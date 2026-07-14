//! Wires a USB-attached phone into the Android Auto stack: runs the AOAP handshake (via
//! `UsbAoapBridge`), dials the resulting USB<->TCP loopback, and drives an AA `Session` over it.
//! Video frames get decoded and rendered via the `gst_video` pipeline; audio channels are played
//! out via `AudioOutput` (one `gst-launch-1.0` PCM sink per channel).

use std::collections::HashMap;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::audio::audio_output::{AudioOutput, AudioOutputOptions};
use crate::video::gst_video::{probe_gst_codecs, GstVideo, GstVideoCodec, VideoRuntime};

use super::session_handle::AaSessionHandle;
use super::stack::aoap::constants::{AOAP_LOOPBACK_HOST, AOAP_LOOPBACK_PORT};
use super::stack::constants::ch;
use super::stack::session::config::{SessionConfig, VideoCodec};
use super::stack::session::session::{Session, SessionEvent};
use super::stack::transport::usb_aoap_bridge::{BridgeEvent, UsbAoapBridge};

/// Matches the rates `Session::handle_av_setup_request` negotiates per channel.
fn audio_format_for(channel_id: u8) -> (i32, i32) {
    if channel_id == ch::MEDIA_AUDIO {
        (48000, 2)
    } else {
        (16000, 1)
    }
}

fn to_gst_codec(codec: VideoCodec) -> GstVideoCodec {
    match codec {
        VideoCodec::H264 => GstVideoCodec::H264,
        VideoCodec::H265 => GstVideoCodec::H265,
        VideoCodec::Vp9 => GstVideoCodec::VP9,
        VideoCodec::Av1 => GstVideoCodec::AV1,
    }
}

/// The AA video is a separate native/compositor surface, not part of the main window's webview
/// content — focusing the main window (opaque) would raise it above that surface and hide the
/// video. This transparent, always-on-top, chrome-less window exists solely to receive input
/// focus and capture touch events instead (see `src/pages/aa-touch.vue`).
///
/// Goes borderless-fullscreen on the monitor, same as "main" (see tauri.conf.json) and the AA
/// video plane the external compositor renders. There's exactly one physical screen on the
/// target device, so matching windows against each other's reported geometry was never the
/// right approach — every window just covers all of it.
pub(crate) fn ensure_touch_window(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    if let Some(window) = app.get_webview_window("aa-touch") {
        return Some(window);
    }

    tauri::WebviewWindowBuilder::new(app, "aa-touch", tauri::WebviewUrl::App("/aa-touch".into()))
        .title("aa-touch")
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .fullscreen(true)
        .build()
        .inspect_err(|e| eprintln!("[AA wired] failed to create aa-touch window: {e}"))
        .ok()
}

/// "main"'s actual physical size, now that it's resized to fill the monitor at startup (see
/// lib.rs). Fed into `SessionConfig::display_width/height` so the SDR computes real letterbox
/// margins for our screen's aspect ratio — without it, the phone assumes its 16:9
/// `video_width`/`video_height` tier exactly matches the display, which stretches AA's video
/// into whatever (differently-shaped) window we actually give it.
fn display_size(app: &AppHandle) -> Option<(u32, u32)> {
    let monitor = app.get_webview_window("main")?.current_monitor().ok()??;
    let size = monitor.size();
    Some((size.width, size.height))
}

pub async fn connect_wired(app: AppHandle, phone: nusb::DeviceInfo) {
    let handle = app.state::<Arc<AaSessionHandle>>().inner().clone();

    let bridge = UsbAoapBridge::new();
    let (bridge_tx, mut bridge_rx) = mpsc::unbounded_channel();

    if let Err(e) = bridge.start(phone, AOAP_LOOPBACK_PORT, bridge_tx).await {
        eprintln!("[AA wired] failed to start USB<->TCP bridge: {e}");
        return;
    }

    // Wait for the bridge's loopback listener to come up, then dial it and hand the socket to
    // a Session. The bridge must be explicitly stopped once we're done with it — dropping the
    // handle does not cancel its background task (a JoinHandle drop just detaches it), which
    // would otherwise leave its TcpListener on the loopback port bound forever, breaking the
    // next connect attempt with "Address already in use".
    'bridge_wait: {
        let event = tokio::select! {
            event = bridge_rx.recv() => event,
            () = handle.shutdown_notify().notified() => break 'bridge_wait,
        };
        let Some(event) = event else { break 'bridge_wait };

        match event {
            BridgeEvent::Ready { host, port } => {
                println!("[AA wired] USB bridge ready on {host}:{port}, dialing loopback");
                match TcpStream::connect((AOAP_LOOPBACK_HOST, AOAP_LOOPBACK_PORT)).await {
                    Ok(socket) => {
                        let probe = probe_gst_codecs(&app);
                        let (display_width, display_height) = display_size(&app).unzip();
                        let cfg = SessionConfig {
                            hevc_supported: probe.h265.hw || probe.h265.sw,
                            vp9_supported: probe.vp9.hw || probe.vp9.sw,
                            av1_supported: probe.av1.hw || probe.av1.sw,
                            display_width,
                            display_height,
                            ..SessionConfig::default()
                        };
                        let session = Session::new(socket, cfg);
                        let (session_tx, session_rx) = mpsc::unbounded_channel();
                        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
                        handle.set(cmd_tx).await;
                        tokio::spawn(handle_session_events(app.clone(), session_rx));
                        session.run(session_tx, cmd_rx, handle.shutdown_notify()).await;
                        handle.clear().await;
                        println!("[AA wired] session ended");
                    }
                    Err(e) => eprintln!("[AA wired] failed to dial loopback: {e}"),
                }
            }
            BridgeEvent::Error(e) => {
                eprintln!("[AA wired] USB bridge error: {e}");
            }
            BridgeEvent::Closed => {}
        }
    }

    bridge.stop().await;
}

/// Consumes session events: decodes+renders the main display's video, logs everything else.
async fn handle_session_events(app: AppHandle, mut rx: mpsc::UnboundedReceiver<SessionEvent>) {
    let mut video: Option<GstVideo> = None;
    // Tracks whether the video plane is currently shown, so it can be hidden while the phone is
    // showing its own host UI (otherwise the last projected frame just freezes there, visible)
    // and shown again once fresh frames resume.
    let mut video_visible = true;
    // One AudioOutput per active audio channel (MEDIA/SPEECH/SYSTEM can all be live at once).
    let mut audio: HashMap<u8, AudioOutput> = HashMap::new();
    // Crop-out-letterbox-bars-and-scale-to-fill region from the SDR negotiation, applied once
    // "video" exists. Arrives well before the first frame (sent during service discovery), so
    // it's normally applied right at video creation below — stashed here in case it ever arrives
    // late instead.
    let mut pending_geometry: Option<(f64, f64, f64, f64, f64, f64)> = None;

    while let Some(event) = rx.recv().await {
        match event {
            SessionEvent::VideoGeometry { crop_left, crop_top, vis_width, vis_height, tier_width, tier_height } => {
                let region = (
                    crop_left as f64,
                    crop_top as f64,
                    vis_width as f64,
                    vis_height as f64,
                    tier_width as f64,
                    tier_height as f64,
                );
                if let Some(v) = video.as_mut() {
                    v.set_content_region(region.0, region.1, region.2, region.3, region.4, region.5).await;
                }
                pending_geometry = Some(region);
            }
            SessionEvent::VideoFrame { channel_id, codec, data, .. } => {
                if channel_id != ch::VIDEO {
                    continue; // cluster display not wired up yet
                }
                let is_first_frame = video.is_none();
                if is_first_frame {
                    let Some(window) = app.get_webview_window("main") else {
                        eprintln!("[AA wired] no \"main\" window to render video into");
                        continue;
                    };
                    let runtime = app.state::<Arc<VideoRuntime>>().inner().clone();
                    let mut new_video = GstVideo::new(runtime, window, "android-auto", "main");
                    if let Some(region) = pending_geometry {
                        new_video.set_content_region(region.0, region.1, region.2, region.3, region.4, region.5).await;
                    }
                    video = Some(new_video);
                }
                if let Some(v) = video.as_mut() {
                    if !video_visible {
                        v.set_visible(&app, true).await;
                        video_visible = true;
                        // Frames resumed after a host-ui detour (see HostUiRequested below),
                        // which only hides the touch overlay rather than closing it — bring it
                        // back now so touch input works again post-resume.
                        if let Some(window) = ensure_touch_window(&app) {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    v.push(&app, to_gst_codec(codec), &data).await;
                }
                if is_first_frame {
                    // Only create the touch overlay *after* gst-host's video surface exists
                    // (rather than at Connected, before any video) — a newly-mapped window is
                    // more likely to raise above an already-fullscreen surface than one that's
                    // just re-asserting focus/always-on-top on an existing mapping.
                    if let Some(window) = ensure_touch_window(&app) {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }

                    // Creating the video player maps a new native surface (waylandsink, via
                    // gst-host), which now also requests fullscreen directly on Linux when no
                    // external compositor is present (see gst_video.cc) — a state transition
                    // that can both steal focus and promote the sink above other clients in
                    // stacking order while it settles. There's no signal for "surface mapped and
                    // fullscreen settled", so keep reclaiming both focus and always-on-top for
                    // longer than a plain (non-fullscreen) window needed before.
                    let app_for_focus = app.clone();
                    tokio::spawn(async move {
                        for _ in 0..30 {
                            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                            if let Some(window) = app_for_focus.get_webview_window("aa-touch") {
                                let _ = window.set_always_on_top(true);
                                let _ = window.set_focus();
                            }
                        }
                    });
                }
            }
            SessionEvent::AudioStart { channel_id } => {
                let (sample_rate, channels) = audio_format_for(channel_id);
                let mut output = AudioOutput::new(AudioOutputOptions {
                    sample_rate,
                    channels,
                    mode: None,
                    device: None,
                });
                output.start(&app).await;
                audio.insert(channel_id, output);
            }
            SessionEvent::AudioFrame { channel_id, data, .. } => {
                if let Some(output) = audio.get_mut(&channel_id) {
                    let samples: Vec<i16> = data
                        .chunks_exact(2)
                        .map(|b| i16::from_le_bytes([b[0], b[1]]))
                        .collect();
                    output.write(samples).await;
                }
            }
            SessionEvent::AudioStop { channel_id } => {
                if let Some(mut output) = audio.remove(&channel_id) {
                    output.stop().await;
                }
            }
            SessionEvent::Connected => {
                // The touch overlay is created later, on the first video frame (see
                // SessionEvent::VideoFrame) — after gst-host's surface exists, not before.
                let _ = app.emit("aa-status", "connected");
            }
            SessionEvent::HostUiRequested => {
                println!("[AA wired] HostUiRequested: closing touch, hiding video, focusing main");
                // No AA video to touch right now — get rid of the overlay so it doesn't sit
                // there (invisibly) intercepting clicks meant for the main window's resume
                // button. hide() alone isn't reliably applied on this WM (confirmed: touch
                // events kept reaching it after hide() returned Ok(())) — close it instead;
                // ensure_touch_window() already lazily recreates it once video resumes.
                let close_result = app.get_webview_window("aa-touch").map(|w| w.close());
                println!("[AA wired] aa-touch close(): {close_result:?}");
                // Otherwise the last projected frame just stays frozen on screen, visible
                // underneath the resume button, until the phone starts sending fresh frames.
                if let Some(v) = video.as_mut() {
                    v.set_visible(&app, false).await;
                    video_visible = false;
                }
                // Hiding the (focused) touch overlay doesn't guarantee focus lands back on
                // "main" — without this, clicks on the resume button can land nowhere.
                let focus_result = app.get_webview_window("main").map(|w| w.set_focus());
                println!("[AA wired] main set_focus(): {focus_result:?}");
                let emit_result = app.emit("aa-status", "host-ui");
                println!("[AA wired] emit aa-status=host-ui: {emit_result:?}");
            }
            SessionEvent::Disconnected => {
                // Also emitted unconditionally after the loop below, which covers every exit
                // path (including the channel just closing); nothing further to do here.
            }
            other => {
                println!("[AA wired] session event: {other:?}");
            }
        }
    }

    let _ = app.emit("aa-status", "disconnected");

    if let Some(mut v) = video {
        v.dispose(&app).await;
    }
    for (_, mut output) in audio.drain() {
        output.stop().await;
    }
    if let Some(window) = app.get_webview_window("aa-touch") {
        let _ = window.close();
    }
}

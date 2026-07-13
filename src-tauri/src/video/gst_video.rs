use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use tauri::Manager;
use tokio::{io::AsyncWriteExt, net::UnixStream, sync::Mutex};

use crate::video::gst_host::{self, GstHost, GstHostHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GstVideoCodec {
    H264,
    H265,
    VP9,
    AV1,
}

impl GstVideoCodec {
    fn as_str(self) -> &'static str {
        match self {
            GstVideoCodec::H264 => "h264",
            GstVideoCodec::H265 => "h265",
            GstVideoCodec::VP9 => "vp9",
            GstVideoCodec::AV1 => "av1",
        }
    }
}

// Parse "#rrggbb" into 0..255 channels, falls back to black on a malformed value.
fn hex_to_rgb255(hex: Option<&str>) -> (u8, u8, u8) {
    let s = hex.unwrap_or("").trim();
    let s = s.strip_prefix('#').unwrap_or(s);

    if s.len() != 6 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return (0, 0, 0);
    }

    let n = u32::from_str_radix(s, 16).unwrap_or(0);
    (
        ((n >> 16) & 0xff) as u8,
        ((n >> 8) & 0xff) as u8,
        (n & 0xff) as u8,
    )
}

// Resolve the active backdrop colour for a config, falling back to the theme defaults.
pub fn backdrop_hex(dark_mode: bool, dark: Option<&str>, light: Option<&str>) -> String {
    let chosen = if dark_mode { dark } else { light };
    match chosen.filter(|s| !s.is_empty()) {
        Some(hex) => hex.to_string(),
        None => (if dark_mode { "#000000" } else { "#d4d4d4" }).to_string(),
    }
}

// Linux only: the pipeline lives in the gst-host process, with its own GLib main loop (so
// waylandsink resizes live) and isolated from a decoder crash. mac/Windows render in-process.
const fn use_host_process() -> bool {
    cfg!(target_os = "linux")
}

// mac/Windows only: point the in-process GStreamer at the app's bundled runtime, once.
fn prepare_runtime(app: &tauri::AppHandle) {
    static PREPARED: std::sync::Once = std::sync::Once::new();
    PREPARED.call_once(|| {
        let root = if cfg!(target_os = "windows") {
            crate::audio::gstreamer::resolve_gstreamer_root(app)
        } else if cfg!(target_os = "macos") {
            // Only override when a bundled runtime is actually present; otherwise keep using
            // whatever GStreamer install the dev machine already has (e.g. Homebrew).
            crate::audio::gstreamer::resolve_gstreamer_root(app).filter(|r| r.exists())
        } else {
            None
        };
        let Some(root) = root else { return };

        // SAFETY: called once, from `Once::call_once`, before any in-process GStreamer pipeline
        // is created (gst_init reads these on first use), so there is no concurrent env access.
        unsafe {
            std::env::set_var("GST_PLUGIN_SYSTEM_PATH", "");
            std::env::set_var("GST_PLUGIN_PATH", root.join("lib").join("gstreamer-1.0"));
            let scanner_name = if cfg!(windows) {
                "gst-plugin-scanner.exe"
            } else {
                "gst-plugin-scanner"
            };
            std::env::set_var(
                "GST_PLUGIN_SCANNER",
                root.join("libexec").join("gstreamer-1.0").join(scanner_name),
            );
            if cfg!(windows) {
                let mut path = std::ffi::OsString::from(root.join("bin"));
                if let Some(existing) = std::env::var_os("PATH") {
                    path.push(";");
                    path.push(existing);
                }
                std::env::set_var("PATH", path);
            }
        }
    });
}

// Which codecs the loaded GStreamer can decode on this platform, and whether hardware
// acceleration is available for each.
pub fn probe_gst_codecs(app: &tauri::AppHandle) -> gst_video::CodecProbe {
    prepare_runtime(app);
    gst_video::probe_codecs()
}

#[derive(Debug, Clone, Copy)]
pub struct GammaState {
    pub gamma: f64,
    pub contrast: f64,
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl GammaState {
    const NEUTRAL: GammaState = GammaState {
        gamma: 1.0,
        contrast: 1.0,
        r: 1.0,
        g: 1.0,
        b: 1.0,
    };
}

// Linux only: control channel to avio-compositor. Video planes are addressed by tag (claim),
// then placed (videocfg) and toggled (videoshow). `state` is resent on reconnect.
#[derive(Default)]
struct CompositorControl {
    path: Option<String>,
    sock: Option<UnixStream>,
    state: HashMap<String, String>, // resent every flush
    outbox: Vec<String>,            // one-shot lines, sent once
}

impl CompositorControl {
    fn new() -> Self {
        Self {
            path: std::env::var("AVIO_COMPOSITOR_CTRL")
                .ok()
                .filter(|s| !s.is_empty()),
            ..Default::default()
        }
    }

    fn enabled(&self) -> bool {
        use_host_process() && self.path.is_some()
    }

    async fn flush(&mut self) {
        if !self.enabled() {
            self.outbox.clear();
            return;
        }
        if self.sock.is_none() {
            if let Some(path) = &self.path {
                self.sock = UnixStream::connect(path).await.ok();
            }
        }
        let Some(sock) = self.sock.as_mut() else {
            return;
        };
        for line in self.outbox.drain(..) {
            if sock.write_all(line.as_bytes()).await.is_err() {
                self.sock = None;
                return;
            }
        }
        for line in self.state.values() {
            if sock.write_all(line.as_bytes()).await.is_err() {
                self.sock = None;
                return;
            }
        }
    }

    // The next new video toplevel gets this tag. Send before creating the waylandsink.
    async fn claim(&mut self, tag: &str) {
        if !self.enabled() {
            return;
        }
        self.outbox.push(format!("claim {tag}\n"));
        self.flush().await;
    }

    // Place + crop the tagged plane on a screen (fullscreen with its own AA content region).
    #[allow(clippy::too_many_arguments)]
    async fn videocfg(
        &mut self,
        tag: &str,
        screen: &str,
        crop_l: f64,
        crop_t: f64,
        vis_w: f64,
        vis_h: f64,
        tier_w: f64,
        tier_h: f64,
    ) {
        if !self.enabled() {
            return;
        }
        let n = |v: f64| v.round() as i64;
        self.state.insert(
            format!("cfg:{tag}"),
            format!(
                "videocfg {tag} {screen} {} {} {} {} {} {}\n",
                n(crop_l),
                n(crop_t),
                n(vis_w),
                n(vis_h),
                n(tier_w),
                n(tier_h)
            ),
        );
        self.flush().await;
    }

    // Toggle the tagged plane's visibility.
    async fn videoshow(&mut self, tag: &str, visible: bool) {
        if !self.enabled() {
            return;
        }
        self.state.insert(
            format!("show:{tag}"),
            format!("videoshow {tag} {}\n", visible as u8),
        );
        self.flush().await;
    }

    // Open/close a role's nested output (its own movable host window). Resent on reconnect.
    async fn screen(&mut self, role: &str, on: bool, size: Option<(f64, f64)>) {
        if !self.enabled() {
            return;
        }
        let size = match size {
            Some((w, h)) if w > 0.0 && h > 0.0 => {
                format!(" {} {}", w.round() as i64, h.round() as i64)
            }
            _ => String::new(),
        };
        self.state.insert(
            format!("screen:{role}"),
            format!("screen {role} {}{size}\n", on as u8),
        );
        self.flush().await;
    }

    // Theme background for the compositor backdrop, hex "#rrggbb" from config.
    async fn set_backdrop(&mut self, hex: &str) {
        if !self.enabled() {
            return;
        }
        let (r, g, b) = hex_to_rgb255(Some(hex));
        self.state
            .insert("__backdrop__".to_string(), format!("backdrop {r} {g} {b}\n"));
        self.flush().await;
    }

    // Push the display calibration to the compositor's per-video shader pass.
    async fn gamma(&mut self, g: GammaState) {
        if !self.enabled() {
            return;
        }
        self.state.insert(
            "__gamma__".to_string(),
            format!("gamma {} {} {} {} {}\n", g.gamma, g.contrast, g.r, g.g, g.b),
        );
        self.flush().await;
    }

    // Ask the compositor to relaunch its inner UI child. One-shot, not resent on reconnect.
    async fn restart(&mut self) -> bool {
        if !self.enabled() {
            return false;
        }
        self.outbox.push("restart\n".to_string());
        self.flush().await;
        true
    }
}

/// Shared video runtime: the out-of-process gst-host handle (Linux), the avio-compositor
/// control channel (Linux), and the current display calibration. One instance for the whole
/// app; `app.manage()` it once and pull it out of `tauri::State` wherever a video surface
/// (a window, or eventually an Android Auto/CarPlay projection surface) needs to drive GStreamer.
pub struct VideoRuntime {
    gst_host: GstHostHandle,
    compositor: Mutex<CompositorControl>,
    gamma: Mutex<GammaState>,
}

impl Default for VideoRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoRuntime {
    pub fn new() -> Self {
        Self {
            gst_host: gst_host::new_handle(),
            compositor: Mutex::new(CompositorControl::new()),
            gamma: Mutex::new(GammaState::NEUTRAL),
        }
    }

    pub async fn set_compositor_backdrop(&self, hex: &str) {
        self.compositor.lock().await.set_backdrop(hex).await;
    }

    pub async fn set_compositor_screen(&self, role: &str, on: bool, size: Option<(f64, f64)>) {
        self.compositor.lock().await.screen(role, on, size).await;
    }

    pub async fn compositor_restart(&self) -> bool {
        self.compositor.lock().await.restart().await
    }

    pub async fn current_gamma(&self) -> GammaState {
        *self.gamma.lock().await
    }

    // Push the display calibration into every live video pipeline's glshader pass (all
    // platforms). Callers holding a live `GstVideo` should call `apply_gamma` on it afterwards
    // to push the new values into its in-process player immediately.
    pub async fn set_stream_gamma(&self, g: GammaState) {
        *self.gamma.lock().await = g;
        if use_host_process() {
            self.compositor.lock().await.gamma(g).await;
        }
    }

    /// Kills the gst-host child process, if one is running.
    pub async fn shutdown(&self) {
        GstHost::shutdown(&self.gst_host).await;
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ContentRegion {
    crop_l: f64,
    crop_t: f64,
    vis_w: f64,
    vis_h: f64,
    tier_w: f64,
    tier_h: f64,
}

static NEXT_PLAYER_ID: AtomicU32 = AtomicU32::new(1);

/// GStreamer video player. On Linux the pipeline lives in the gst-host process and this only
/// holds an id for it; on mac/Windows it drives the in-process gst-video crate directly.
///
/// `role` is the compositor tag for this plane (Linux); `target_screen` is which screen it's
/// placed on.
pub struct GstVideo {
    id: u32,
    runtime: Arc<VideoRuntime>,
    window: tauri::WebviewWindow,
    role: String,
    target_screen: String,
    started: bool,
    player: Option<gst_video::Player>,
    codec: Option<GstVideoCodec>,
    visible: bool,
    region: Option<ContentRegion>,
}

impl GstVideo {
    pub fn new(
        runtime: Arc<VideoRuntime>,
        window: tauri::WebviewWindow,
        role: impl Into<String>,
        target_screen: impl Into<String>,
    ) -> Self {
        Self {
            id: NEXT_PLAYER_ID.fetch_add(1, Ordering::Relaxed),
            runtime,
            window,
            role: role.into(),
            target_screen: target_screen.into(),
            started: false,
            player: None,
            codec: None,
            visible: true,
            region: None,
        }
    }

    fn window_handle(&self) -> u64 {
        #[cfg(target_os = "macos")]
        {
            self.window
                .window()
                .ns_view()
                .map(|p| p as u64)
                .unwrap_or(0)
        }
        #[cfg(windows)]
        {
            self.window
                .window()
                .hwnd()
                .map(|h| h.0 as u64)
                .unwrap_or(0)
        }
        #[cfg(not(any(target_os = "macos", windows)))]
        {
            0
        }
    }

    async fn ensure(&mut self, app: &tauri::AppHandle, codec: GstVideoCodec) {
        if use_host_process() {
            if self.started && self.codec == Some(codec) {
                return;
            }
            self.dispose(app).await;
            // Tag the waylandsink toplevel the host process creates next.
            self.runtime.compositor.lock().await.claim(&self.role).await;
            GstHost::create_player(&self.runtime.gst_host, app, self.id, codec.as_str()).await;
            self.codec = Some(codec);
            self.started = true;
            self.apply_gamma().await;
            return;
        }

        prepare_runtime(app);
        if self.player.as_ref().is_some_and(|p| p.is_dead()) {
            // The sink lost its output surface; drop it so the check below recreates a fresh one.
            self.player = None;
            self.codec = None;
        }
        if self.player.is_some() && self.codec == Some(codec) {
            return;
        }
        self.dispose(app).await;
        let handle = self.window_handle();
        if handle == 0 {
            return;
        }
        self.runtime.compositor.lock().await.claim(&self.role).await;
        let Some(mut player) = gst_video::Player::create(codec.as_str(), handle) else {
            return;
        };
        player.start();
        player.set_visible(self.visible);
        if let Some(r) = self.region {
            player.set_content_region(r.crop_l, r.crop_t, r.vis_w, r.vis_h, r.tier_w, r.tier_h);
        }
        self.player = Some(player);
        self.codec = Some(codec);
        self.apply_gamma().await;
    }

    pub async fn push(&mut self, app: &tauri::AppHandle, codec: GstVideoCodec, nal: &[u8]) {
        self.ensure(app, codec).await;
        if use_host_process() {
            if self.started {
                GstHost::push_buffer(&self.runtime.gst_host, app, self.id, nal).await;
            }
            return;
        }
        if let Some(player) = self.player.as_mut() {
            player.push_buffer(nal);
        }
    }

    // Apply the current calibration to this player's glshader pass. Re-sent after each (re)create.
    pub async fn apply_gamma(&mut self) {
        let g = self.runtime.current_gamma().await;
        if use_host_process() {
            self.runtime.compositor.lock().await.gamma(g).await;
            return;
        }
        if let Some(player) = self.player.as_mut() {
            player.set_gamma(g.gamma, g.contrast, g.r, g.g, g.b);
        }
    }

    // Show/hide the video surface as the user navigates in/out of projection.
    //
    // On Linux (host_process mode) without avio-compositor running, `videoshow` below is a
    // no-op (see CompositorControl::enabled) and there's no in-process `self.player` either —
    // meaning hiding had *no effect at all* in that setup: the last decoded frame just stayed
    // on screen frozen. Tear the gst-host player down instead when hiding without a compositor;
    // `push()` transparently recreates it (via `ensure()`) once frames resume.
    pub async fn set_visible(&mut self, app: &tauri::AppHandle, visible: bool) {
        self.visible = visible;
        self.runtime
            .compositor
            .lock()
            .await
            .videoshow(&self.role, visible)
            .await;
        if let Some(player) = self.player.as_mut() {
            player.set_visible(visible);
        }
        if use_host_process() && !visible && self.started {
            GstHost::stop(&self.runtime.gst_host, app, self.id).await;
            self.started = false;
            self.codec = None;
        }
    }

    // Set the AA content region inside the decoded tier. The native view crops to it by
    // sizing + positioning the GL render (zero-copy).
    #[allow(clippy::too_many_arguments)]
    pub async fn set_content_region(
        &mut self,
        crop_l: f64,
        crop_t: f64,
        vis_w: f64,
        vis_h: f64,
        tier_w: f64,
        tier_h: f64,
    ) {
        self.region = (vis_w > 0.0 && vis_h > 0.0).then_some(ContentRegion {
            crop_l,
            crop_t,
            vis_w,
            vis_h,
            tier_w,
            tier_h,
        });
        // Linux: the compositor places + crops the tagged plane on its target screen.
        self.runtime
            .compositor
            .lock()
            .await
            .videocfg(
                &self.role,
                &self.target_screen,
                crop_l,
                crop_t,
                vis_w,
                vis_h,
                tier_w,
                tier_h,
            )
            .await;
        if let Some(player) = self.player.as_mut() {
            let r = self.region.unwrap_or_default();
            player.set_content_region(r.crop_l, r.crop_t, r.vis_w, r.vis_h, r.tier_w, r.tier_h);
        }
    }

    pub async fn dispose(&mut self, app: &tauri::AppHandle) {
        if use_host_process() {
            if self.started {
                GstHost::stop(&self.runtime.gst_host, app, self.id).await;
            }
            self.started = false;
            self.codec = None;
            return;
        }
        if let Some(mut player) = self.player.take() {
            player.stop();
        }
        self.codec = None;
    }
}

// macOS only: paint the window's content view (below the video subviews) with the theme colour.
pub fn set_mac_backdrop(window: &tauri::WebviewWindow, hex: &str) {
    #[cfg(target_os = "macos")]
    {
        let Ok(handle) = window.window().ns_view() else {
            return;
        };
        if handle.is_null() {
            return;
        }
        let (r, g, b) = hex_to_rgb255(Some(hex));
        gst_video::set_backdrop(
            handle as u64,
            r as f64 / 255.0,
            g as f64 / 255.0,
            b as f64 / 255.0,
        );
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, hex);
    }
}

// A short (2s, 640x480, all-intra so any loop restart lands on a keyframe) H264 Annex-B test
// clip, baked into the binary. Encoders aren't part of this app's bundled GStreamer at all (it
// only ever decodes an already-encoded source, e.g. from a phone), so generating the test
// pattern on the fly isn't an option the way it would be with a full desktop GStreamer install.
// `profile=high` matters: without forcing it, x264enc's auto-selected profile for an all-intra
// encode is a 4:4:4/10-bit "high444 intra" profile hardware decoders (e.g. VA-API) reject with
// not-negotiated, even though software decode (avdec_h264) tolerates it fine.
// Regenerate with:
//   gst-launch-1.0 videotestsrc pattern=smpte num-buffers=48 \
//     ! video/x-raw,format=I420,width=640,height=480,framerate=24/1 \
//     ! x264enc tune=zerolatency speed-preset=ultrafast key-int-max=1 bitrate=400 \
//     ! video/x-h264,profile=high \
//     ! h264parse config-interval=-1 ! video/x-h264,stream-format=byte-stream,alignment=au \
//     ! filesink location=src-tauri/media/sample.h264
const SAMPLE_H264: &[u8] = include_bytes!("../../media/sample.h264");

/// Loops a baked-in H264 test clip over the app's existing main window through this module's
/// real path: repeated [`GstVideo::push`] calls, exactly like a real decoded source would
/// arrive. Demonstrates the plumbing; not meant to be the final test-pattern UI.
///
/// This deliberately does *not* open a new window. `GstVideo` was never meant to own a
/// dedicated window: on Linux it only needs a `role` tag (avio-compositor places that plane by
/// tag, over/under whatever's already on screen — see [`CompositorControl`]), and on
/// mac/Windows it attaches a native overlay to whichever window you hand it. The original
/// Electron code passed it the `WebContents` of the window already showing the app's UI (or a
/// projection view within it) for exactly this reason. A second window here would just be an
/// independent surface with no relationship to the one actually being driven.
pub async fn open_gst_test_window(app: tauri::AppHandle) -> Result<(), String> {
    let runtime = app.state::<Arc<VideoRuntime>>().inner().clone();
    let window = app
        .get_webview_window("main")
        .ok_or("no \"main\" window")?;

    let mut video = GstVideo::new(runtime, window, "gst-test", "main");

    tauri::async_runtime::spawn(async move {
        loop {
            // Push in chunks with a little pacing rather than one shot, closer to how a real
            // streamed source arrives.
            for chunk in SAMPLE_H264.chunks(4096) {
                video.push(&app, GstVideoCodec::H264, chunk).await;
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        }
    });

    Ok(())
}

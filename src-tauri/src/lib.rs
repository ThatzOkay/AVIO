use std::sync::Arc;

use evno::Bus;
use tauri::Manager;
use tokio::sync::Mutex;

use crate::{radio::radio_service::RadioService, usb::usb_service::{self, UsbService}};

pub mod audio;
pub mod video;
pub mod projection;
pub mod radio;
pub mod shared;
pub mod usb;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn open_gst_test_window(app: tauri::AppHandle) -> Result<(), String> {
    video::gst_video::open_gst_test_window(app).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app: &mut tauri::App| {
            let app_handle = app.handle().clone();

            let event_bus = Bus::new(128);

            app.manage(event_bus.clone());

            let usb_service = UsbService::new(&app_handle);

            let radio_service = Arc::new(Mutex::new(RadioService::new(&app_handle)));

            app.manage(usb_service.clone());
            app.manage(radio_service.clone());
            app.manage(Arc::new(video::gst_video::VideoRuntime::new()));
            app.manage(Arc::new(projection::driver::aa::session_handle::AaSessionHandle::default()));

            let radio_service_init = radio_service.clone();
            tauri::async_runtime::spawn(async move {
                let mut radio = radio_service_init.lock().await;
                radio.init();
            });

            // Signaled once "main" has finished its resize-to-monitor + fullscreen-settle dance
            // below. If a phone is already plugged in at cold start, UsbService::init()'s scan
            // can find and connect it almost immediately — racing the AA video/touch windows'
            // creation against "main" still mid-transition, which changes final stacking order
            // versus a hotplug later (when "main" has long since settled). Gating USB start on
            // this fixes that: a hotplug always happens on an already-stable desktop either way.
            let main_settled = Arc::new(tokio::sync::Notify::new());

            let app_for_udev = app_handle.clone();
            let main_settled_for_usb = main_settled.clone();
            tauri::async_runtime::spawn(async move {
                usb::udev_rule::check_and_install_udev_rule(&app_for_udev).await;
                main_settled_for_usb.notified().await;

                let mut service = usb_service.lock().await;
                service.init().await;
                UsbService::start(usb_service.clone());
            });

            // Tauri only fires RunEvent::Exit once *every* window is closed. The "aa-touch"
            // window (see wired_driver.rs) is separate from "main", so closing just "main"
            // would otherwise leave the app running headless with aa-touch still open and
            // RunEvent::Exit never firing at all. Force a full exit on "main" closing instead.
            //
            // NB: the window's label is "main" (tauri.conf.json's windows[0] has no explicit
            // "label", and Tauri defaults that to "main") — it must match here and everywhere
            // else this window is looked up (wired_driver.rs, gst_video.rs). A mismatch here
            // makes get_webview_window() return None silently, skipping this whole block
            // (including the resize-to-monitor logic below) with no visible error.
            match app.get_webview_window("main") {
                Some(main_window) => {
                    let window_for_resize = main_window.clone();
                    tauri::async_runtime::spawn(async move {
                        let mut settled = false;
                        for _ in 0..20 {
                            if let Ok(Some(monitor)) = window_for_resize.current_monitor() {
                                let _ = window_for_resize.set_position(tauri::Position::Physical(*monitor.position()));
                                let _ = window_for_resize.set_size(tauri::Size::Physical(*monitor.size()));
                                let _ = window_for_resize.set_fullscreen(false);
                                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                                let _ = window_for_resize.set_fullscreen(true);
                                settled = true;
                                break;
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                        }
                        if !settled {
                            eprintln!("[lib] gave up waiting for \"main\" to be mapped to a monitor; falling back to maximize()");
                            let _ = window_for_resize.maximize();
                        }
                        main_settled.notify_one();
                    });

                    let app_for_close = app_handle.clone();
                    main_window.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { .. } = event {
                            app_for_close.exit(0);
                        }
                    });
                }
                None => {
                    eprintln!("[lib] no \"main\" window found in setup() — check tauri.conf.json's window label");
                }
            }

            Ok(())
        })
        .plugin(tauri_plugin_rtl_sdr::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            audio::list_sinks,
            audio::list_sources,
            radio::start,
            radio::stop,
            radio::get_fm_state,
            radio::set_fm_frequency,
            radio::step_fm,
            radio::set_fm_favorite,
            radio::recall_fm_favorite,
            open_gst_test_window,
            projection::driver::aa::commands::aa_send_touch,
            projection::driver::aa::commands::aa_resume
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                let video_runtime = app_handle.state::<Arc<video::gst_video::VideoRuntime>>().inner().clone();
                let aa_session = app_handle
                    .state::<Arc<projection::driver::aa::session_handle::AaSessionHandle>>()
                    .inner()
                    .clone();
                tauri::async_runtime::block_on(async move {
                    // Signal the wired driver to stop and release the USB device/loopback port,
                    // then give it a moment to actually do that before tearing down video.
                    aa_session.request_shutdown();
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                    video_runtime.shutdown().await;
                });
            }
        });
}

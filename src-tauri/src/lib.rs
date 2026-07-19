use std::sync::Arc;

use evno::Bus;
use tauri::Manager;
use tokio::sync::Mutex;

use crate::{radio::radio_service::RadioService, usb::usb_service::UsbService};

pub mod audio;
pub mod projection;
pub mod radio;
pub mod screen;
pub mod shared;
pub mod usb;
pub mod video;

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
    let context = tauri::generate_context!();
    if video::compositor_bootstrap::maybe_bootstrap(&context) {
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app: &mut tauri::App| {
            // Note: the main window's background color is NOT set transparent here - it's
            // toggled at runtime by aa_set_main_transparent, in sync with App.vue's `show-video`
            // class. Setting it once and unconditionally would defeat WebKitGTK's normal opaque
            // default for the whole app, not just AA video mode.

            let app_handle = app.handle().clone();

            let event_bus = Bus::new(128);

            app.manage(event_bus.clone());

            let usb_service = UsbService::new(&app_handle);

            let radio_service = Arc::new(Mutex::new(RadioService::new(&app_handle)));

            app.manage(usb_service.clone());
            app.manage(radio_service.clone());
            app.manage(Arc::new(video::gst_video::VideoRuntime::new()));
            app.manage(Arc::new(
                projection::driver::aa::session_handle::AaSessionHandle::default(),
            ));

            let radio_service_init = radio_service.clone();
            tauri::async_runtime::spawn(async move {
                println!("Initializing radio service");
                let mut radio = radio_service_init.lock().await;
                radio.init();
            });

            let app_for_udev = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                println!("Checking and installing udev rule for USB devices");
                usb::udev_rule::check_and_install_udev_rule(&app_for_udev).await;

                let mut service = usb_service.lock().await;
                service.init().await;
                println!("USB service initialized");
                UsbService::start(usb_service.clone());
            });

            Ok(())
        })
        .plugin(tauri_plugin_rtl_sdr::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            audio::list_sinks,
            audio::list_sources,
            audio::get_current_volume,
            audio::get_default_device_name,
            audio::set_current_volume,
            screen::get_current_brightness,
            screen::set_brightness,
            radio::start,
            radio::stop,
            radio::get_fm_state,
            radio::set_fm_frequency,
            radio::step_fm,
            radio::set_fm_favorite,
            radio::recall_fm_favorite,
            open_gst_test_window,
            projection::driver::aa::commands::aa_send_touch,
            projection::driver::aa::commands::aa_resume,
            projection::driver::aa::commands::aa_set_main_transparent
        ])
        .build(context)
        .expect("error while running tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                let video_runtime = app_handle
                    .state::<Arc<video::gst_video::VideoRuntime>>()
                    .inner()
                    .clone();
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

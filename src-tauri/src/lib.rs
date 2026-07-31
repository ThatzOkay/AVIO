use std::sync::Arc;

use evno::Bus;
use tauri::Manager;
use tokio::sync::Mutex;
use tauri_plugin_store::StoreExt;

use crate::{radio::radio_service::RadioService, state::SettingsState, state::AppSettings, usb::usb_service::UsbService};

pub mod audio;
pub mod projection;
pub mod radio;
pub mod screen;
pub mod shared;
pub mod usb;
pub mod video;
pub mod state;
pub mod window;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn get_settings(state: tauri::State<'_, SettingsState>) -> Result<AppSettings, String> {
    let settings = state.0.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
async fn save_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, SettingsState>,
    settings: AppSettings,
) -> Result<(), String> {
    let store = app.store("settings.json").map_err(|e| e.to_string())?;

    let mut current_settings = state.0.lock().map_err(|e| e.to_string())?;
    *current_settings = settings;

    store.set("master_volume", current_settings.master_volume);
    store.set("default_sound_device", current_settings.default_sound_device.clone());
    store.set("default_input_device", current_settings.default_input_device.clone());
    store.set("brightness", current_settings.brightness);
    store.set("theme", current_settings.theme.clone());
    store.set("seed", current_settings.seed.clone());
    store.set("scale", current_settings.scale);
    store.set("ao_resolution", current_settings.ao_resolution.clone());
    store.set("ao_framerate", current_settings.ao_framerate.clone());
    store.set("ao_dpi", current_settings.ao_dpi);
    store.set("ao_view_area_top", current_settings.ao_view_area_top);
    store.set("ao_view_area_bottom", current_settings.ao_view_area_bottom);
    store.set("ao_view_area_left", current_settings.ao_view_area_left);
    store.set("ao_view_area_right", current_settings.ao_view_area_right);
    store.set("ao_safe_area_top", current_settings.ao_safe_area_top);
    store.set("ao_safe_area_bottom", current_settings.ao_safe_area_bottom);
    store.set("ao_safe_area_left", current_settings.ao_safe_area_left);
    store.set("ao_safe_area_right", current_settings.ao_safe_area_right);
    store.set("kiosk", current_settings.kiosk);
    store.set("display_mode", current_settings.display_mode.clone());
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn open_gst_test_window(app: tauri::AppHandle) -> Result<(), String> {
    video::gst_video::open_gst_test_window(app).await
}

/// Resolution/refresh modes the host display currently offers - see `window::display_mode`.
/// Empty on non-Linux, or wherever there's no separate host session / `wlr-randr` to query.
#[tauri::command]
fn list_display_modes() -> Vec<String> {
    window::display_mode::list_host_output_modes()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();
    if video::compositor_bootstrap::maybe_bootstrap(&context) {
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
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

            let win = app.get_webview_window("main").unwrap();
            let _ = win.eval("window.location.reload()");

            let app_handle = app.handle().clone();

            let store = app.store("settings.json")?;
            let state = SettingsState(std::sync::Mutex::new(AppSettings {
                master_volume: store
                    .get("master_volume")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u8)
                    .unwrap_or_default(),
                default_sound_device: store
                    .get("default_sound_device")
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default(),
                default_input_device: store
                    .get("default_input_device")
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default(),
                brightness: store
                    .get("brightness")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u8)
                    .unwrap_or_default(),
                theme: store
                    .get("theme")
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default(),
                seed: store
                    .get("seed")
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default(),
                scale: store
                    .get("scale")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i8)
                    .unwrap_or_default(),
                ao_resolution: store
                    .get("ao_resolution")
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default(),
                ao_framerate: store
                    .get("ao_framerate")
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default(),
                ao_dpi: store
                    .get("ao_dpi")
                    .and_then(|v| v.as_f64())
                    .unwrap_or_default(),
                ao_view_area_top: store
                    .get("ao_view_area_top")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or_default(),
                ao_view_area_bottom: store
                    .get("ao_view_area_bottom")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or_default(),
                ao_view_area_left: store
                    .get("ao_view_area_left")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or_default(),
                ao_view_area_right: store
                    .get("ao_view_area_right")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or_default(),
                ao_safe_area_top: store
                    .get("ao_safe_area_top")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or_default(),
                ao_safe_area_bottom: store
                    .get("ao_safe_area_bottom")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or_default(),
                ao_safe_area_left: store
                    .get("ao_safe_area_left")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or_default(),
                ao_safe_area_right: store
                    .get("ao_safe_area_right")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or_default(),
                kiosk: store
                    .get("kiosk")
                    .and_then(|v| v.as_bool())
                    .unwrap_or_default(),
                display_mode: store
                    .get("display_mode")
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default(),
            }));

            let (kiosk_setting, display_mode) = {
                let settings = state.0.lock().unwrap();
                (settings.kiosk, settings.display_mode.clone())
            };

            app.manage(state);

            if let Some(main_window) = app.get_webview_window("main") {
                window::setup_main_window(&main_window, kiosk_setting, &display_mode);
            }

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
            get_settings,
            save_settings,
            audio::list_sinks,
            audio::list_sources,
            audio::get_current_volume,
            audio::get_default_device_name,
            audio::set_current_volume,
            audio::get_audio_devices,
            audio::set_default_device,
            audio::set_mute,
            audio::get_mute,
            audio::get_input_devices,
            audio::set_default_input_device,
            audio::get_default_input_device_name,
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
            list_display_modes,
            projection::driver::aa::commands::aa_send_pointer,
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

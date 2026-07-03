use std::sync::Arc;

use evno::Bus;
use tauri::Manager;
use tokio::sync::Mutex;

use crate::{radio::radio_service::RadioService, usb::usb_service::{self, UsbService}};

pub mod audio;
pub mod video;
pub mod radio;
pub mod shared;
pub mod usb;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
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

            let radio_service_init = radio_service.clone();
            tauri::async_runtime::spawn(async move {
                let mut radio = radio_service_init.lock().await;
                radio.init();
            });

            tauri::async_runtime::spawn(async move {
                let mut service = usb_service.lock().await;
                service.init().await;
                UsbService::start(usb_service.clone());
            });

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
            radio::recall_fm_favorite
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

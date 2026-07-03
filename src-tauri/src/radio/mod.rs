use std::sync::Arc;

use tauri::Manager;
use tokio::sync::Mutex;

use crate::radio::radio_service::RadioState;

pub mod fm_radio_service;
pub mod radio_service;

#[tauri::command]
pub async fn start(app: tauri::AppHandle, frequency: u32) -> Result<RadioState, String> {
    let radio_service = app.state::<Arc<Mutex<radio_service::RadioService>>>();
    let mut radio_service = radio_service.lock().await;
    let mode = radio_service.mode.clone();
    let state = match mode {
        radio_service::RadioMode::FM => radio_service.start_fm(frequency).await,
        radio_service::RadioMode::DAB => return Err("DAB is not implemented yet".to_string()),
    };
    Ok(state)
}

#[tauri::command]
pub async fn stop(app: tauri::AppHandle) -> Result<RadioState, String> {
    let radio_service = app.state::<Arc<Mutex<radio_service::RadioService>>>();
    let mut radio_service = radio_service.lock().await;
    let mode = radio_service.mode.clone();
    let state = match mode {
        radio_service::RadioMode::FM => radio_service.stop_fm().await,
        radio_service::RadioMode::DAB => return Err("DAB is not implemented yet".to_string()),
    };
    Ok(state)
}

#[tauri::command]
pub async fn get_fm_state(app: tauri::AppHandle) -> Result<RadioState, String> {
    let radio_service = app.state::<Arc<Mutex<radio_service::RadioService>>>();
    let radio_service = radio_service.lock().await;
    Ok(radio_service.get_fm_state())
}

#[tauri::command]
pub async fn set_fm_frequency(app: tauri::AppHandle, frequency: u32) -> Result<RadioState, String> {
    let radio_service = app.state::<Arc<Mutex<radio_service::RadioService>>>();
    let mut radio_service = radio_service.lock().await;
    Ok(radio_service.set_fm_frequency(frequency).await)
}

#[tauri::command]
pub async fn step_fm(app: tauri::AppHandle, direction: i32, fast: bool) -> Result<RadioState, String> {
    let radio_service = app.state::<Arc<Mutex<radio_service::RadioService>>>();
    let mut radio_service = radio_service.lock().await;
    Ok(radio_service.step_fm(direction, fast).await)
}

#[tauri::command]
pub async fn set_fm_favorite(app: tauri::AppHandle, slot: usize) -> Result<RadioState, String> {
    let radio_service = app.state::<Arc<Mutex<radio_service::RadioService>>>();
    let mut radio_service = radio_service.lock().await;
    Ok(radio_service.set_fm_favorite(slot).await)
}

#[tauri::command]
pub async fn recall_fm_favorite(app: tauri::AppHandle, slot: usize) -> Result<RadioState, String> {
    let radio_service = app.state::<Arc<Mutex<radio_service::RadioService>>>();
    let mut radio_service = radio_service.lock().await;
    Ok(radio_service.recall_fm_favorite(slot).await)
}

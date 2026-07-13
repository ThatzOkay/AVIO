//! Tauri commands the frontend uses to send input to whichever AA session is running.

use std::sync::Arc;

use tauri::{AppHandle, State};

use super::session_handle::AaSessionHandle;
use super::stack::channels::input_channel::touch_action;
use super::stack::session::session::SessionCommand;
use super::wired_driver::ensure_touch_window;

// Matches SessionConfig::default()'s advertised touchscreen tier until real per-session
// settings exist.
const TOUCH_W: f64 = 1280.0;
const TOUCH_H: f64 = 720.0;

/// `x`/`y` are normalised 0..1 coordinates within the rendered video area. `phase` is
/// "down"/"move"/"up".
#[tauri::command]
pub async fn aa_send_touch(
    handle: State<'_, Arc<AaSessionHandle>>,
    x: f64,
    y: f64,
    phase: String,
) -> Result<(), String> {
    let action = match phase.as_str() {
        "down" => touch_action::DOWN,
        "move" => touch_action::MOVED,
        "up" => touch_action::UP,
        other => return Err(format!("unknown touch phase: {other}")),
    };
    let px = (x.clamp(0.0, 1.0) * TOUCH_W).round() as u32;
    let py = (y.clamp(0.0, 1.0) * TOUCH_H).round() as u32;
    println!("[AA touch] sending {action:?} at {px},{py} (norm {x:.3},{y:.3})");
    handle.send(SessionCommand::Touch { action, x: px, y: py }).await;
    Ok(())
}

/// Resumes projected AA content after the phone kicked the display back to its native/host UI
/// (see the "aa-status" event with payload "host-ui").
#[tauri::command]
pub async fn aa_resume(app: AppHandle, handle: State<'_, Arc<AaSessionHandle>>) -> Result<(), String> {
    // Eagerly reopen the touch overlay right on click rather than only reacting to the next AA
    // video frame — the phone resuming projection depends on it actually receiving and honoring
    // our RequestVideoFocus, which isn't guaranteed, so the touch surface shouldn't be stuck
    // waiting on that round trip. ensure_touch_window() is a no-op if it's already open.
    let touch_result = ensure_touch_window(&app).map(|w| {
        let _ = w.show();
        let _ = w.set_focus();
    });
    println!("[AA resume] ensure_touch_window: {}", touch_result.is_some());

    let reached_session = handle.send(SessionCommand::RequestVideoFocus).await;
    println!("[AA resume] RequestVideoFocus sent, reached a live session: {reached_session}");
    Ok(())
}

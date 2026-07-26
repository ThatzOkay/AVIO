//! Tauri commands the frontend uses to send input to whichever AA session is running.

use std::sync::Arc;

use tauri::{AppHandle, Manager, State};

use super::session_handle::AaSessionHandle;
use super::stack::channels::input_channel::{button_key, touch_action};
use super::stack::session::session::SessionCommand;

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
    handle
        .send(SessionCommand::Touch {
            action,
            x: px,
            y: py,
        })
        .await;
    Ok(())
}

/// Resumes projected AA content after the phone kicked the display back to its native/host UI
/// (see the "aa-status" event with payload "host-ui").
///
/// A bare `RequestVideoFocus` isn't reliably honored once the phone is holding NATIVE focus -
/// pairing it with a HOME button press first more reliably relinquishes that focus back to
/// projection.
#[tauri::command]
pub async fn aa_resume(handle: State<'_, Arc<AaSessionHandle>>) -> Result<(), String> {
    let key_codes = [button_key::HOME];
    handle
        .send(SessionCommand::Button {
            key_codes: key_codes.to_vec(),
            down: true,
            longpress: false,
        })
        .await;
    handle
        .send(SessionCommand::Button {
            key_codes: key_codes.to_vec(),
            down: false,
            longpress: false,
        })
        .await;
    let reached_session = handle.send(SessionCommand::RequestVideoFocus).await;
    println!(
        "[AA resume] HOME + RequestVideoFocus sent, reached a live session: {reached_session}"
    );
    Ok(())
}

/// Toggles the main window's background color in lockstep with App.vue's `show-video` class.
/// WebKitGTK needs both the CSS `background: transparent` and an explicit zero-alpha window
/// background to actually punch through to the AA video plane underneath — but only while
/// projecting. Leaving the window's own background permanently transparent (rather than
/// toggling it here) defeats WebKitGTK's normal opaque default for the rest of the app too,
/// wherever Vuetify's own CSS doesn't happen to paint something explicit.
#[tauri::command]
pub fn aa_set_main_transparent(app: AppHandle, transparent: bool) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("no \"main\" window")?;
    let color = transparent.then_some(tauri::window::Color(0, 0, 0, 0));
    window
        .set_background_color(color)
        .map_err(|e| e.to_string())
}

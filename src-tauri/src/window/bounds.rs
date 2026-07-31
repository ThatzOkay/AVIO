//! Persisted main-window geometry for windowed (non-kiosk) mode: saved on move/resize, restored
//! (and sanitized against whatever displays are actually connected) on the next launch. Mirrors
//! LIVI's `sanitizeBounds` and bounds-persistence in `window/utils.ts`/`createWindow.ts`.
//!
//! Kept in its own store key (not `SettingsState`/`AppSettings`) since this is a backend-only
//! geometry cache with no user-facing setting to wire up - unlike `kiosk`/`display_mode`.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_store::StoreExt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

// A saved rect only counts as "still on-screen" if it overlaps some connected monitor's work
// area by at least this many pixels in both dimensions - guards against the window opening
// off-screen after a monitor was unplugged or the desktop layout changed since the last run.
const MIN_VISIBLE: i32 = 64;

const STORE_KEY: &str = "mainWindowBounds";

pub fn load_bounds(app: &AppHandle) -> Option<WindowBounds> {
    let store = app.store("settings.json").ok()?;
    let value = store.get(STORE_KEY)?;
    serde_json::from_value(value.clone()).ok()
}

pub fn save_bounds(app: &AppHandle, bounds: WindowBounds) {
    let Ok(store) = app.store("settings.json") else {
        return;
    };
    store.set(STORE_KEY, serde_json::to_value(bounds).unwrap_or_default());
    let _ = store.save();
}

/// Discards `bounds` unless it overlaps some connected monitor's work area by at least
/// `MIN_VISIBLE` pixels in both dimensions. Trusts the saved rect if no monitor info is
/// available at all (rather than discarding a perfectly good position on a query failure).
pub fn sanitize_bounds(bounds: WindowBounds, monitors: &[tauri::Monitor]) -> Option<WindowBounds> {
    if monitors.is_empty() {
        return Some(bounds);
    }
    let fits = monitors.iter().any(|m| {
        let wa = m.work_area();
        let overlap_w = (bounds.x + bounds.width as i32).min(wa.position.x + wa.size.width as i32)
            - bounds.x.max(wa.position.x);
        let overlap_h = (bounds.y + bounds.height as i32)
            .min(wa.position.y + wa.size.height as i32)
            - bounds.y.max(wa.position.y);
        overlap_w >= MIN_VISIBLE && overlap_h >= MIN_VISIBLE
    });
    fits.then_some(bounds)
}

/// Saves the window's current position/size on move/resize, debounced (500ms, matching LIVI) so
/// a drag doesn't spam disk writes. Skipped while fullscreen - kiosk geometry isn't a "position"
/// worth remembering for the next windowed launch.
pub fn attach_bounds_persistence(window: &WebviewWindow) {
    let win = window.clone();
    let pending: Arc<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>> = Arc::default();

    window.on_window_event(move |event| {
        if !matches!(
            event,
            tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_)
        ) {
            return;
        }
        if win.is_fullscreen().unwrap_or(false) {
            return;
        }

        let win = win.clone();
        let mut guard = pending.lock().unwrap();
        if let Some(handle) = guard.take() {
            handle.abort();
        }
        *guard = Some(tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let (Ok(pos), Ok(size)) = (win.outer_position(), win.inner_size()) else {
                return;
            };
            save_bounds(
                win.app_handle(),
                WindowBounds {
                    x: pos.x,
                    y: pos.y,
                    width: size.width,
                    height: size.height,
                },
            );
        }));
    });
}

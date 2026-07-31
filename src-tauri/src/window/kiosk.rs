//! Kiosk/fullscreen mode: whether we want it, and re-entering it if the window manager (or a
//! stray keypress) knocks the window out of fullscreen. Mirrors LIVI's
//! `attachKioskStateSync`/`restoreKioskAfterWmExit` (window/utils.ts) - Linux WMs don't reliably
//! emit a dedicated "you left fullscreen" signal the way macOS does, so this reacts to other
//! window events instead.

use tauri::WebviewWindow;

/// Whether kiosk mode is wanted, from either the persisted setting or an env var override -
/// mirrors LIVI's `cfg.kiosk?.main === true || process.env.LIVI_KIOSK === '1'` check.
pub fn want_kiosk(kiosk_setting: bool) -> bool {
    kiosk_setting || std::env::var("AVIO_KIOSK").as_deref() == Ok("1")
}

/// Puts the window into fullscreen kiosk mode.
pub fn enter_kiosk(window: &WebviewWindow) {
    let _ = window.set_fullscreen(true);
}

/// Re-asserts fullscreen if something took the window out of it. Only does anything when kiosk
/// mode is actually wanted - otherwise this would fight a legitimate windowed session.
pub fn attach_self_heal(window: &WebviewWindow, kiosk_wanted: bool) {
    if !kiosk_wanted {
        return;
    }
    let win = window.clone();
    window.on_window_event(move |event| {
        if !matches!(
            event,
            tauri::WindowEvent::Resized(_)
                | tauri::WindowEvent::Moved(_)
                | tauri::WindowEvent::Focused(true)
        ) {
            return;
        }
        if !win.is_fullscreen().unwrap_or(true) {
            let _ = win.set_fullscreen(true);
        }
    });
}

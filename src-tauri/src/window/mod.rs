//! Main-window sizing/placement: kiosk vs windowed startup, persisted bounds (sanitized against
//! whatever displays are actually connected), self-healing fullscreen, and host display-mode
//! switching before going kiosk. Mirrors LIVI's `window/` + `app/hostOutput.ts`.

pub mod bounds;
pub mod display_mode;
pub mod kiosk;
pub mod sizing;

pub use bounds::WindowBounds;

use tauri::{Manager, WebviewWindow};

/// Called once from `run()`'s `.setup()`, after the "main" window already exists. Decides
/// kiosk-vs-windowed startup layout and wires up persistence/self-heal for whichever applies.
pub fn setup_main_window(window: &WebviewWindow, kiosk_setting: bool, display_mode: &str) {
    let want_kiosk = kiosk::want_kiosk(kiosk_setting);

    if want_kiosk {
        if !display_mode.is_empty() {
            display_mode::apply_host_output_mode(display_mode);
        }
        kiosk::enter_kiosk(window);
    } else {
        let restored = bounds::load_bounds(window.app_handle()).and_then(|b| {
            let monitors = window.available_monitors().unwrap_or_default();
            bounds::sanitize_bounds(b, &monitors)
        });
        match restored {
            Some(b) => {
                let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                    x: b.x,
                    y: b.y,
                }));
                sizing::apply_windowed_content_size(window, b.width, b.height);
            }
            None => sizing::apply_windowed_content_size(window, 1280, 720),
        }
        bounds::attach_bounds_persistence(window);
    }

    kiosk::attach_self_heal(window, want_kiosk);
}

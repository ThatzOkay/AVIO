//! Clamps a requested window content size to the work area of whichever monitor the window
//! currently sits on. Mirrors LIVI's `applyWindowedContentSize` (window/utils.ts) - without this,
//! a configured or persisted windowed size could end up requesting more than a smaller screen
//! actually has to give (the same class of bug as the nested-compositor oversized-window issue,
//! just for plain windowed mode instead).

use tauri::WebviewWindow;

/// Clamps `(w, h)` down to `monitor`'s work area, never below 1px.
pub fn clamp_to_workarea(w: u32, h: u32, monitor: &tauri::Monitor) -> (u32, u32) {
    let max_w = monitor.work_area().size.width.max(1);
    let max_h = monitor.work_area().size.height.max(1);
    (w.clamp(1, max_w), h.clamp(1, max_h))
}

/// Applies `(w, h)` as the window's content size, clamped to whichever monitor it currently
/// overlaps (falling back to the primary monitor, or the unclamped size if neither is known).
pub fn apply_windowed_content_size(window: &WebviewWindow, w: u32, h: u32) {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    let (w, h) = match &monitor {
        Some(m) => clamp_to_workarea(w, h, m),
        None => (w, h),
    };
    let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize { width: w, height: h }));
}

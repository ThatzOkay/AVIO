//! Lets a kiosk deployment pick the *host* display's native resolution/refresh rate before going
//! fullscreen. Tries KDE's `kscreen-doctor` first (talks to the kscreen daemon over the session
//! D-Bus, so it works regardless of which nested Wayland display this process is itself a client
//! of), then falls back to `wlr-randr` against the host Wayland compositor for wlroots-based
//! hosts (Sway/Hyprland/Cage/labwc - KWin and Mutter don't implement the `wlr-output-management`
//! protocol `wlr-randr` needs, confirmed via `wayland-info` against a live KDE session). Mirrors
//! LIVI's `hostOutput.ts`, extended with the KDE path LIVI doesn't have.
//!
//! A no-op wherever neither tool is available/applicable - e.g. when `avio-compositor` took over
//! DRM directly instead of nesting (see `compositor_bootstrap.rs`), there's no host session at
//! all; the kernel's own preferred mode (`detect_output_size` there) is what applies instead.

mod exec;
mod kscreen;
mod wlr_randr;

/// Lists distinct "WIDTHxHEIGHT" modes the host's primary output offers, widest-area-first.
pub fn list_host_output_modes() -> Vec<String> {
    let modes = kscreen::list_modes();
    if !modes.is_empty() {
        return modes;
    }
    wlr_randr::list_modes()
}

/// Applies `mode` ("WIDTHxHEIGHT") to the host's primary output, if it's one of the modes that
/// output actually offers. No-op (logged) if neither backend is available, or the requested mode
/// isn't one the host offers.
pub fn apply_host_output_mode(mode: &str) {
    if mode
        .split_once('x')
        .is_none_or(|(w, h)| w.parse::<u32>().is_err() || h.parse::<u32>().is_err())
    {
        eprintln!("[display_mode] invalid mode string: {mode}");
        return;
    }
    if kscreen::apply_mode(mode) || wlr_randr::apply_mode(mode) {
        return;
    }
    eprintln!("[display_mode] no backend could apply mode {mode} (not offered, or neither kscreen-doctor nor wlr-randr are usable)");
}

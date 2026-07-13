use std::path::PathBuf;
use std::process::Command;

/// Linux only: spawn the nested `avio-compositor` and re-exec this same binary as its inner
/// Wayland client (its `-s` flag, see avio-compositor.c's getopt handling — forks+execs the
/// given command via `/bin/sh -c`, inheriting our env plus the new nested `WAYLAND_DISPLAY`).
///
/// Must run before `tauri::Builder` ever touches GTK/WebKit: tauri creates the configured
/// "main" window as part of `Builder::build()`, before `.setup()` runs, so there's no later
/// hook to prevent this (outer) process from creating a real window. Call this at the very top
/// of `run()` and `return` immediately if it returns `true` — the window that matters lives in
/// the re-exec'd child once it's running inside the nested display, not in this process.
#[cfg(target_os = "linux")]
pub fn maybe_bootstrap(context: &tauri::Context<tauri::Wry>) -> bool {
    if std::env::var("AVIO_COMPOSITOR").as_deref() == Ok("1") {
        return false; // already the nested child
    }
    if std::env::var("AVIO_NO_COMPOSITOR").as_deref() == Ok("1") {
        return false; // opted out (desktop dev convenience)
    }

    #[cfg(target_os = "linux")]
    if running_under_real_compositor() {
        eprintln!("[compositor] detected real compositor; skipping nested bootstrap");
        return false;
    }

    let Some(launcher) = launcher_path(context) else {
        eprintln!("[compositor] avio-compositor bundle not found; running without it");
        return false;
    };

    let Ok(relaunch) = std::env::current_exe() else {
        eprintln!("[compositor] could not resolve current_exe() to relaunch inside the compositor");
        return false;
    };

    // GDK_BACKEND=wayland: without it GTK could fall back to XWayland instead of the nested
    // display. AVIO_COMPOSITOR=1 short-circuits this same check on the re-exec'd instance.
    let inner = format!(
        "AVIO_COMPOSITOR=1 GDK_BACKEND=wayland '{}'",
        relaunch.display()
    );

    let ctrl_sock =
        std::env::temp_dir().join(format!("avio-compositor-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&ctrl_sock);

    // Must match the Wayland app_id the re-exec'd "main" window actually presents, so the
    // compositor recognizes it as the UI client rather than a generic (zero-sized) dialog — see
    // avio-compositor.c's AVIO_OUTPUT_APP_ID comparison. Confirmed empirically (not
    // `identifier`/"nl.thatzokay.avio" — GTK's xdg_toplevel app_id falls back to the package
    // name regardless of `app.enableGTKAppId`): the toplevel shows up as `app_id='avio'`.
    let app_id = context.package_info().name.clone();

    let mut command = Command::new(&launcher);
    command
        .arg("-s")
        .arg(&inner)
        .env("AVIO_COMPOSITOR_CTRL", &ctrl_sock)
        .env("AVIO_SCREENS", "main")
        .env("AVIO_OUTPUT_APP_ID", &app_id);

    // Without this the compositor's nested output defaults to 1280x720 (avio-compositor.c's
    // AVIO_OUTPUT_SIZE fallback), so the "main" window's later resize-to-monitor dance in
    // lib.rs's setup() dutifully fills that undersized output instead of the real screen.
    if let Some((w, h)) = detect_output_size() {
        command.env("AVIO_OUTPUT_SIZE", format!("{w}x{h}"));
    }

    let spawn_result = command.spawn();

    let mut child = match spawn_result {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[compositor] failed to spawn {}: {e}", launcher.display());
            return false;
        }
    };

    // Block here rather than returning right away: `tauri dev` watches this process's exit as
    // the signal that the app quit, and tears down the whole dev session (killing the compositor
    // mid-startup) the instant we return. Waiting for the compositor's own exit — which only
    // happens once its nested "-s" child (the re-exec'd avio instance) closes — makes this
    // process's lifetime match a normal single-process Tauri app's from the CLI's point of view.
    match child.wait() {
        Ok(status) => {
            if !status.success() {
                eprintln!("[compositor] avio-compositor exited: {status}");
            }
        }
        Err(e) => eprintln!("[compositor] failed to wait on avio-compositor: {e}"),
    }
    true
}

#[cfg(not(target_os = "linux"))]
pub fn maybe_bootstrap(_context: &tauri::Context<tauri::Wry>) -> bool {
    false
}

/// Returns true if we're running under a known "real" Wayland compositor
/// (KWin, GNOME Mutter, Hyprland, etc.) where spawning our own nested
/// compositor would be redundant / unwanted.
#[cfg(target_os = "linux")]
fn running_under_real_compositor() -> bool {
    // XDG_CURRENT_DESKTOP covers most DEs
    if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
        let desktop = desktop.to_lowercase();
        if [
            "kde", "gnome", "hyprland", "sway", "wayfire", "labwc", "river",
        ]
        .iter()
        .any(|d| desktop.contains(d))
        {
            return true;
        }
    }

    // Fallback: check if a known compositor process is running
    if let Ok(output) = std::process::Command::new("pgrep")
        .args(["-x", "kwin_wayland,mutter,hyprland,sway,wayfire"])
        .output()
    {
        if output.status.success() {
            return true;
        }
    }

    false
}

/// Reads the first connected DRM output's preferred mode straight from sysfs (kernel-reported
/// modelines are listed highest-preferred-first), so we don't need a live X11/Wayland connection
/// — this runs before the outer process has one. Picks the first connected output found; fine
/// for a kiosk box with exactly one display, not meant to disambiguate a multi-monitor desktop.
#[cfg(target_os = "linux")]
fn detect_output_size() -> Option<(u32, u32)> {
    for entry in std::fs::read_dir("/sys/class/drm").ok()?.flatten() {
        let path = entry.path();
        let status = std::fs::read_to_string(path.join("status")).unwrap_or_default();
        if status.trim() != "connected" {
            continue;
        }
        let Ok(modes) = std::fs::read_to_string(path.join("modes")) else {
            continue;
        };
        let Some(first_mode) = modes.lines().next() else {
            continue;
        };
        let Some((w, h)) = first_mode.split_once('x') else {
            continue;
        };
        if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) {
            return Some((w, h));
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn launcher_path(context: &tauri::Context<tauri::Wry>) -> Option<PathBuf> {
    // Packaged: tauri's bundler drops `bundle.resources` under the platform resource dir
    // (AppImage: $APPDIR/usr/lib/<name>/, deb/rpm: ../lib/<name>/ next to the exe) — see
    // scripts/build-avio-compositor-sidecar.ts, which builds the `compositor/` tree that
    // tauri.conf.json's `bundle.resources` picks up.
    if let Ok(resource_dir) =
        tauri::utils::platform::resource_dir(context.package_info(), &tauri::utils::Env::default())
    {
        let launcher = resource_dir.join("compositor").join("avio-compositor");
        if launcher.exists() {
            return Some(launcher);
        }
    }

    // Dev: the build script writes straight into the source tree (it's a whole bin+lib tree,
    // not a single relocatable sidecar next to our own exe like gst-host).
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("compositor")
        .join("avio-compositor");
    dev.exists().then_some(dev)
}

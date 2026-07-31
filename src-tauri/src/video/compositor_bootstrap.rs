#[cfg(target_os = "linux")]
use std::path::PathBuf;
#[cfg(target_os = "linux")]
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

    // Only trust the DRM-probed physical size on bare metal (no host session at all — the
    // compositor's own `wlr_backend_autocreate()` picks the real DRM/KMS backend in that case).
    // Once nested inside an existing desktop (WAYLAND_DISPLAY/DISPLAY set), the host places our
    // toplevel in ITS OWN logical/scaled coordinate space, and wlroots' wl_backend has no buffer
    // scale handling at all — requesting the panel's raw physical pixel count there just makes
    // the window come out `scale` times too big on any HiDPI desktop (confirmed against a KDE
    // session at 1.35x scale). LIVI never probes for a physical size either; it just defaults to
    // 1280x720 (`resizable: true` lets you size it yourself), so do the same here rather than
    // fighting the host compositor's scaling.
    let host_wayland_display = std::env::var_os("WAYLAND_DISPLAY");
    let nested = host_wayland_display.is_some() || std::env::var_os("DISPLAY").is_some();
    if !nested {
        if let Some((w, h)) = detect_output_size() {
            command.env("AVIO_OUTPUT_SIZE", format!("{w}x{h}"));
        }
    }

    // Lets `window::display_mode` (running inside the re-exec'd child, whose own
    // WAYLAND_DISPLAY now points at the *nested* compositor) still reach the host session to
    // query/switch its output mode via `wlr-randr` — see that module for why.
    if let Some(host_display) = &host_wayland_display {
        command.env("AVIO_HOST_WAYLAND_DISPLAY", host_display);
    }

    let spawn_result = command.spawn();

    let mut child = match spawn_result {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[compositor] failed to spawn {}: {e}", launcher.display());
            return false;
        }
    };

    // `tauri dev`'s file watcher restarts on every source change by killing this (outer) process
    // directly, with no chance for our own cleanup code to run. Without a handler, avio-compositor
    // (and its own re-exec'd inner UI child) would just be orphaned rather than torn down, piling
    // up one more live instance on every single hot-reload. Kill it ourselves first.
    let child_pid = child.id();
    std::thread::spawn(move || {
        let Ok(mut signals) =
            signal_hook::iterator::Signals::new([signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT])
        else {
            return;
        };
        if signals.forever().next().is_some() {
            unsafe {
                libc::kill(child_pid as libc::pid_t, libc::SIGTERM);
            }
        }
    });

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

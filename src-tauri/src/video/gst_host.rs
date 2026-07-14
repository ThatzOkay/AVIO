// Linux only: the pipeline lives in the gst-host process, with its own GLib main loop (so
// waylandsink resizes live) and isolated from a decoder crash, forwarding calls over a unix
// socket. mac/Windows render in-process instead (see `gst_video::use_host_process`), and tokio
// has no unix-socket support on Windows at all, so the real implementation below is Linux-only
// and every other platform gets the no-op stub further down with the same public API.
#[cfg(target_os = "linux")]
mod linux {
    use std::{path::PathBuf, sync::Arc};

    use tauri::Manager;
    use tokio::{
        io::AsyncWriteExt,
        net::{UnixListener, UnixStream},
        sync::Mutex,
    };

    /// Shared handle to the out-of-process pipeline host. One instance serves every [`super::super::gst_video::GstVideo`]
    /// player on this platform, multiplexed by player id over a single socket.
    pub type GstHostHandle = Arc<Mutex<GstHost>>;

    pub fn new_handle() -> GstHostHandle {
        Arc::new(Mutex::new(GstHost::default()))
    }

    fn crash_log_path(app: &tauri::AppHandle) -> PathBuf {
        let mut path = app.path().app_data_dir().unwrap_or_default();
        path.push("crash.log");
        path
    }

    // Frame: [u32 LE len][u8 op][u32 LE id][rest]. op 1 create(codec), 2 data, 3 stop,
    // 4 setGamma (5 float64).
    fn frame(op: u8, id: u32, rest: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(9 + rest.len());
        buf.extend_from_slice(&(5 + rest.len() as u32).to_le_bytes());
        buf.push(op);
        buf.extend_from_slice(&id.to_le_bytes());
        buf.extend_from_slice(rest);
        buf
    }

    fn host_binary_path() -> Option<PathBuf> {
        let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
        let ext = if cfg!(windows) { ".exe" } else { "" };

        // Packaged: tauri's bundler drops `bundle.externalBin` sidecars next to the main
        // executable, suffixed with the Rust target triple (see scripts/build-gst-host-sidecar.mjs).
        if let Ok(triple) = tauri::utils::platform::target_triple() {
            let sidecar = dir.join(format!("gst-host-{triple}{ext}"));
            if sidecar.exists() {
                return Some(sidecar);
            }
        }

        // Dev: cargo puts the gst-video crate's own bin target in the same target/<profile> dir
        // as our own executable, unsuffixed.
        let dev = dir.join(format!("gst-host{ext}"));
        dev.exists().then_some(dev)
    }

    /// Spawns the gst-video pipeline in the standalone `gst-host` binary (not the app's own
    /// process, which would export a bundled libffi that can corrupt wayland marshalling) and
    /// forwards calls over a unix socket.
    #[derive(Default)]
    pub struct GstHost {
        sock: Option<UnixStream>,
        starting: bool,
        queue: Vec<Vec<u8>>,
        watcher: Option<tauri::async_runtime::JoinHandle<()>>,
    }

    impl GstHost {
        async fn ensure_started(handle: &GstHostHandle, app: &tauri::AppHandle) {
            {
                let mut this = handle.lock().await;
                if this.sock.is_some() || this.starting {
                    return;
                }
                this.starting = true;
            }

            let Some(host_bin) = host_binary_path().filter(|p| p.exists()) else {
                eprintln!("[GstHost] gst-host binary not found next to the app executable");
                handle.lock().await.starting = false;
                return;
            };
            // The exec bit can end up stripped (e.g. a packaging step that copies without
            // preserving permissions), so re-assert it before every spawn attempt.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ =
                    std::fs::set_permissions(&host_bin, std::fs::Permissions::from_mode(0o755));
            }

            let sock_path =
                std::env::temp_dir().join(format!("avio-gst-{}.sock", std::process::id()));
            let crash_path = crash_log_path(app);
            let _ = std::fs::remove_file(&sock_path);
            let _ = std::fs::remove_file(&crash_path);

            let listener = match UnixListener::bind(&sock_path) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[GstHost] bind {} failed: {e}", sock_path.display());
                    handle.lock().await.starting = false;
                    return;
                }
            };

            let mut command = tokio::process::Command::new(&host_bin);
            command
                .arg(&sock_path)
                .arg(&crash_path)
                .env("GST_GL_WINDOW", "surfaceless")
                .env("GST_GL_PLATFORM", "egl")
                .kill_on_drop(true);
            // AVIO_GST_PRELOAD LD_PRELOADs an override lib into the gst-host child only.
            if let Ok(preload) = std::env::var("AVIO_GST_PRELOAD") {
                command.env("LD_PRELOAD", preload);
            }

            let mut child = match command.spawn() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[GstHost] failed to spawn {}: {e}", host_bin.display());
                    handle.lock().await.starting = false;
                    return;
                }
            };

            let sock = match listener.accept().await {
                Ok((s, _)) => s,
                Err(e) => {
                    eprintln!("[GstHost] accept failed: {e}");
                    let _ = child.kill().await;
                    handle.lock().await.starting = false;
                    return;
                }
            };

            {
                let mut this = handle.lock().await;
                this.sock = Some(sock);
                this.starting = false;
                let queued: Vec<Vec<u8>> = this.queue.drain(..).collect();
                if let Some(s) = this.sock.as_mut() {
                    for buf in queued {
                        let _ = s.write_all(&buf).await;
                    }
                }
            }

            let watch_handle = Arc::clone(handle);
            let watcher = tauri::async_runtime::spawn(async move {
                let status = child.wait().await;
                eprintln!("[GstHost] child exited: {status:?}");
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;
                    let crashed = status.as_ref().ok().and_then(|s| s.signal()).is_some();
                    if crashed {
                        if let Ok(contents) = std::fs::read_to_string(&crash_path) {
                            eprintln!(
                                "[GstHost] crash backtrace ({}):\n{contents}",
                                crash_path.display()
                            );
                        }
                    }
                }
                let mut this = watch_handle.lock().await;
                this.sock = None;
                this.starting = false;
            });
            handle.lock().await.watcher = Some(watcher);
        }

        /// Kills the gst-host child (if running) and clears all state. Call this from the app's
        /// exit handler (`tauri::RunEvent::Exit`) so the child doesn't outlive the app: aborting
        /// its watcher task drops the owned `Child`, and `kill_on_drop(true)` (set on spawn) sends
        /// it a kill signal.
        pub async fn shutdown(handle: &GstHostHandle) {
            let mut this = handle.lock().await;
            if let Some(watcher) = this.watcher.take() {
                watcher.abort();
            }
            this.sock = None;
            this.starting = false;
            this.queue.clear();
        }

        async fn send(handle: &GstHostHandle, app: &tauri::AppHandle, buf: Vec<u8>) {
            Self::ensure_started(handle, app).await;
            let mut this = handle.lock().await;
            let ok = match this.sock.as_mut() {
                Some(sock) => sock.write_all(&buf).await.is_ok(),
                None => false,
            };
            if !ok {
                this.sock = None;
                this.queue.push(buf);
            }
        }

        pub async fn create_player(
            handle: &GstHostHandle,
            app: &tauri::AppHandle,
            id: u32,
            codec: &str,
        ) {
            Self::send(handle, app, frame(1, id, codec.as_bytes())).await;
        }

        pub async fn push_buffer(
            handle: &GstHostHandle,
            app: &tauri::AppHandle,
            id: u32,
            nal: &[u8],
        ) {
            Self::send(handle, app, frame(2, id, nal)).await;
        }

        pub async fn stop(handle: &GstHostHandle, app: &tauri::AppHandle, id: u32) {
            Self::send(handle, app, frame(3, id, &[])).await;
        }

        // op 4: calibration LUT as 5 little-endian float64 (gamma, contrast, gain R/G/B).
        #[allow(clippy::too_many_arguments)]
        pub async fn set_gamma(
            handle: &GstHostHandle,
            app: &tauri::AppHandle,
            id: u32,
            gamma: f64,
            contrast: f64,
            r: f64,
            g: f64,
            b: f64,
        ) {
            let mut rest = Vec::with_capacity(40);
            for v in [gamma, contrast, r, g, b] {
                rest.extend_from_slice(&v.to_le_bytes());
            }
            Self::send(handle, app, frame(4, id, &rest)).await;
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::{new_handle, GstHost, GstHostHandle};

// mac/Windows: `gst_video::GstVideo` renders in-process (`use_host_process` is false), so every
// method here is dead weight at runtime - this stub exists only so call sites (in particular
// `VideoRuntime::shutdown`, which calls `GstHost::shutdown` unconditionally on every platform)
// don't need their own cfg-gating.
#[cfg(not(target_os = "linux"))]
mod stub {
    use std::sync::Arc;

    use tokio::sync::Mutex;

    pub type GstHostHandle = Arc<Mutex<GstHost>>;

    pub fn new_handle() -> GstHostHandle {
        Arc::new(Mutex::new(GstHost))
    }

    #[derive(Default)]
    pub struct GstHost;

    impl GstHost {
        pub async fn shutdown(_handle: &GstHostHandle) {}

        pub async fn create_player(
            _handle: &GstHostHandle,
            _app: &tauri::AppHandle,
            _id: u32,
            _codec: &str,
        ) {
        }

        pub async fn push_buffer(
            _handle: &GstHostHandle,
            _app: &tauri::AppHandle,
            _id: u32,
            _nal: &[u8],
        ) {
        }

        pub async fn stop(_handle: &GstHostHandle, _app: &tauri::AppHandle, _id: u32) {}

        #[allow(clippy::too_many_arguments)]
        pub async fn set_gamma(
            _handle: &GstHostHandle,
            _app: &tauri::AppHandle,
            _id: u32,
            _gamma: f64,
            _contrast: f64,
            _r: f64,
            _g: f64,
            _b: f64,
        ) {
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub use stub::{new_handle, GstHost, GstHostHandle};

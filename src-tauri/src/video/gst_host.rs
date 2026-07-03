
fn crash_log_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    let mut path = app.path_resolver().app_data_dir().unwrap_or_default();
    path.push("crash.log");
    path
}

fn frame(op: u8, id: u32, rest: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(9 + rest.len());
    buf.extend_from_slice(&((5 + rest.len()) as u32).to_le_bytes());
    buf.push(op);
    buf.extend_from_slice(&id.to_le_bytes());
    buf.extend_from_slice(rest);
    buf
}

struct GstHost {
    child: Option<std::process::Child>,
    sock: Option<std::os::unix::net::UnixStream>,
    starting: bool,
    quit_hooked: bool,
    queue: Vec<u8>,
}

impl GstHost {
    fn new() -> Self {
        Self {
            child: None,
            sock: None,
            starting: false,
            quit_hooked: false,
            queue: Vec::new(),
        }
    }

    fn start(&mut self, app: &tauri::AppHandle) {
        if self.child.is_some() || self.starting {
            return;
        }

        self.starting = true;

        
    }
}
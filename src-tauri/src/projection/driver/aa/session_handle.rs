//! Handle to whichever AA `Session` is currently running (if any), so Tauri commands invoked
//! from the frontend (touch input, buttons, ...) can reach it without needing a direct reference.
//! `wired_driver` registers/clears it as sessions start and end; only one session runs at a time.

use tokio::sync::{mpsc, Mutex, Notify};

use super::stack::session::session::SessionCommand;

#[derive(Default)]
pub struct AaSessionHandle {
    commands: Mutex<Option<mpsc::UnboundedSender<SessionCommand>>>,
    shutdown: Notify,
}

impl AaSessionHandle {
    pub async fn set(&self, sender: mpsc::UnboundedSender<SessionCommand>) {
        *self.commands.lock().await = Some(sender);
    }

    pub async fn clear(&self) {
        *self.commands.lock().await = None;
    }

    /// Sends a command to the active session, if any. Returns `false` if there's no session
    /// running (or it just ended) — callers can treat that as a silent no-op.
    pub async fn send(&self, command: SessionCommand) -> bool {
        match self.commands.lock().await.as_ref() {
            Some(tx) => tx.send(command).is_ok(),
            None => false,
        }
    }

    /// Tells the wired driver (and whichever session is running) to stop and release the USB
    /// device/loopback port. `notify_one`, not `notify_waiters`: there's exactly one consumer at
    /// a time (the running session, or the bridge's connect loop between sessions), and
    /// `notify_one` stores a permit if it isn't waiting yet, so the signal isn't lost if it
    /// arrives between loop iterations.
    pub fn request_shutdown(&self) {
        self.shutdown.notify_one();
    }

    pub fn shutdown_notify(&self) -> &Notify {
        &self.shutdown
    }
}

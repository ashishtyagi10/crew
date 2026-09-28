//! The checkpoint runs beside the routing, and no tool runs before it lands.
//!
//! Every task used to pin the working tree FIRST — a fresh-index `git add -A`
//! and `write-tree`, measured at ~0.3 s on the crew repo — and only then
//! start deciding what to do. Nothing in that deciding touches a file: the
//! routing call, the context line and the answering model's first tokens all
//! take a second or more before any tool is called. So the snapshot now runs
//! on its own thread from the start of the task, and the one place a file can
//! change — a tool call — waits at this gate until the snapshot is taken.
//!
//! The wait is bounded: a checkpoint that hangs (a cold network mount) delays
//! a tool by [`WAIT_MAX`] at most, never forever — the same "a slow safety net
//! must not become a hang" rule the checkpoint's own git probes follow.
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// Longest a tool call waits for a checkpoint that has not landed.
pub(crate) const WAIT_MAX: Duration = Duration::from_secs(15);

/// Open once the task's checkpoint has been taken (or was never needed).
#[derive(Default)]
pub(crate) struct CkptGate {
    closed: Mutex<bool>,
    cv: Condvar,
}

impl CkptGate {
    /// A gate with nothing to wait for — a session outside any task.
    pub(crate) fn open_now() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// A gate that holds tool calls until [`Self::open`].
    pub(crate) fn pending() -> Arc<Self> {
        let g = Self::default();
        *g.closed.lock().unwrap_or_else(|e| e.into_inner()) = true;
        Arc::new(g)
    }

    /// The checkpoint is taken (or failed — a failure must not hold work up).
    pub(crate) fn open(&self) {
        *self.closed.lock().unwrap_or_else(|e| e.into_inner()) = false;
        self.cv.notify_all();
    }

    /// Wait until open, at most [`WAIT_MAX`].
    pub(crate) fn wait(&self) {
        let g = self.closed.lock().unwrap_or_else(|e| e.into_inner());
        let _ = self.cv.wait_timeout_while(g, WAIT_MAX, |closed| *closed);
    }
}

#[cfg(test)]
#[path = "ckptgate_tests.rs"]
mod tests;

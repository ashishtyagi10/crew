//! The broker's running-task registry: several agent tasks run concurrently,
//! each on its own worker thread with its own cancel flag and a monotonic id.
//! Replaces the old single-worker/`busy`/`cancel` model in `run_broker_stdio`.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

/// One running background task.
///
/// No `label` or `started` here any more: they existed to render `/tasks`,
/// and that construct is gone. The label now rides the `PluginEvent::Task`
/// start event straight to the pane, and the pane has known the start time
/// since it saw that event — storing either here as well would be a second
/// copy of the truth that nothing reads.
struct Task {
    id: u64,
    cancel: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

/// All background tasks currently running.
pub(crate) struct Tasks {
    next_id: u64,
    running: Vec<Task>,
}

impl Tasks {
    pub(crate) fn new() -> Self {
        Tasks {
            next_id: 0,
            running: Vec::new(),
        }
    }

    /// Concurrency cap. `CREW_MAX_TASKS` overrides (default 4, floored at 1).
    pub(crate) fn max() -> usize {
        std::env::var("CREW_MAX_TASKS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4usize)
            .max(1)
    }

    /// Whether another task may start (live count below the cap).
    pub(crate) fn admit(&self) -> bool {
        self.running.len() < Self::max()
    }

    /// Register a spawned task in one step; returns its new id. (Test-only:
    /// `stdio` uses `reserve` + `attach` because it needs the id before the
    /// `JoinHandle` exists, so `register` is dead code in the non-test build.)
    #[cfg(test)]
    pub(crate) fn register(&mut self, cancel: Arc<AtomicBool>, handle: JoinHandle<()>) -> u64 {
        let id = self.reserve();
        self.attach(id, cancel, handle);
        id
    }

    /// Reserve the next id (before a worker/handle exists).
    pub(crate) fn reserve(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    /// Attach a spawned worker to a previously reserved id.
    pub(crate) fn attach(&mut self, id: u64, cancel: Arc<AtomicBool>, handle: JoinHandle<()>) {
        self.running.push(Task { id, cancel, handle });
    }

    /// Drop tasks whose worker thread has exited.
    pub(crate) fn reap(&mut self) {
        self.running.retain(|t| !t.handle.is_finished());
    }

    /// Trip task `id`'s cancel flag; `false` if no such task.
    pub(crate) fn cancel(&self, id: u64) -> bool {
        match self.running.iter().find(|t| t.id == id) {
            Some(t) => {
                t.cancel.store(true, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }

    /// Trip every running task's cancel flag; returns how many.
    pub(crate) fn cancel_all(&self) -> usize {
        for t in &self.running {
            t.cancel.store(true, Ordering::Relaxed);
        }
        self.running.len()
    }

    pub(crate) fn len(&self) -> usize {
        self.running.len()
    }

    /// Join all worker threads (called on stdin EOF so output isn't truncated).
    pub(crate) fn join_all(&mut self) {
        for t in self.running.drain(..) {
            let _ = t.handle.join();
        }
    }
}

#[cfg(test)]
#[path = "tasks_tests.rs"]
mod tests;

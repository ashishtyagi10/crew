//! Mutable per-connection broker state: settings the user changes with slash
//! constructs (per-agent model overrides, …) that must survive across sends
//! for as long as the `/crew` pane is open, plus the shared cancel flag the
//! `/stop` construct trips while a task runs on the worker thread.
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::stdio::{call_timeout, max_hops, token_budget};
use super::{Broker, Registry};

pub(crate) struct Session {
    /// Per-agent model overrides (`agent name → model id`), set by `/model`.
    /// Agents without an entry run their provider default, so different agents
    /// can run different models side by side.
    pub overrides: HashMap<String, String>,
    /// Tripped by `/stop`; long constructs check it between hops/rounds.
    pub cancel: Arc<AtomicBool>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            overrides: HashMap::new(),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// A worker-thread copy: its own override map (reads only), the SAME
    /// cancel flag — so `/stop` on the main loop reaches the running task.
    pub fn snapshot(&self) -> Self {
        Self {
            overrides: self.overrides.clone(),
            cancel: Arc::clone(&self.cancel),
        }
    }

    /// Whether `/stop` has been requested for the running task.
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// The agent registry with this session's model overrides applied.
    pub fn registry(&self) -> Registry {
        Registry::discover_with(&self.overrides)
    }

    /// A relay broker over `reg` with the env knobs and this session's cancel
    /// flag applied — every construct builds its broker here.
    pub fn broker(&self, reg: Registry) -> Broker {
        Broker::new(reg, max_hops(), call_timeout())
            .with_budget(token_budget())
            .with_cancel_flag(Arc::clone(&self.cancel))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_no_overrides_and_not_cancelled() {
        let s = Session::new();
        assert!(s.overrides.is_empty());
        assert!(!s.cancelled());
    }

    #[test]
    fn snapshot_shares_the_cancel_flag() {
        let s = Session::new();
        let snap = s.snapshot();
        s.cancel.store(true, Ordering::Relaxed);
        assert!(snap.cancelled(), "worker sees the main loop's /stop");
    }
}

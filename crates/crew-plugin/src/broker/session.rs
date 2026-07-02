//! Mutable per-connection broker state: settings the user changes with slash
//! constructs (per-agent model overrides, …) that must survive across sends
//! for as long as the `/crew` pane is open, plus the shared cancel flag the
//! `/stop` construct trips while a task runs on the worker thread.
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use std::time::Duration;

use super::{Broker, Registry};

pub(crate) fn max_hops() -> u32 {
    env_num("CREW_BROKER_MAX_HOPS").unwrap_or(6)
}
pub(crate) fn call_timeout() -> Duration {
    Duration::from_millis(env_num("CREW_BROKER_TIMEOUT_MS").unwrap_or(180_000))
}
/// Approximate per-thread token budget (0 = unlimited). `CREW_BROKER_TOKEN_BUDGET`.
pub(crate) fn token_budget() -> usize {
    env_num("CREW_BROKER_TOKEN_BUDGET").unwrap_or(0)
}
fn env_num<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok().and_then(|s| s.parse().ok())
}

pub(crate) struct Session {
    /// Per-agent model overrides (`agent name → model id`), set by `/model`.
    /// Agents without an entry run their provider default, so different agents
    /// can run different models side by side.
    pub overrides: HashMap<String, String>,
    /// Tripped by `/stop`; long constructs check it between hops/rounds.
    pub cancel: Arc<AtomicBool>,
    /// The running construct's label (None = idle), shared with the worker.
    pub busy: Arc<Mutex<Option<String>>>,
    /// Session totals for `/status`: worker tasks started, ~tokens spent.
    pub turns: Arc<AtomicU64>,
    pub tokens: Arc<AtomicU64>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            overrides: HashMap::new(),
            cancel: Arc::new(AtomicBool::new(false)),
            busy: Arc::new(Mutex::new(None)),
            turns: Arc::new(AtomicU64::new(0)),
            tokens: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// A worker-thread copy: its own override map (reads only), the SAME
    /// cancel flag / busy label / counters — so `/stop` on the main loop
    /// reaches the running task and `/status` sees live totals.
    pub fn snapshot(&self) -> Self {
        Self {
            overrides: self.overrides.clone(),
            cancel: Arc::clone(&self.cancel),
            busy: Arc::clone(&self.busy),
            turns: Arc::clone(&self.turns),
            tokens: Arc::clone(&self.tokens),
        }
    }

    /// The running construct's label, if any.
    pub fn running(&self) -> Option<String> {
        self.busy.lock().unwrap_or_else(|e| e.into_inner()).clone()
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

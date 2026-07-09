//! Mutable per-connection broker state: settings the user changes with slash
//! constructs (per-agent model overrides, …) that must survive across sends
//! for as long as the `/crew` pane is open.
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
    /// Session totals for `/status`: worker tasks started, ~tokens spent.
    pub turns: Arc<AtomicU64>,
    pub tokens: Arc<AtomicU64>,
    /// The configured MCP servers, shared with worker snapshots so lazy
    /// connections and the per-server tool cache live once per pane.
    pub mcp: Arc<Mutex<crate::mcp::McpHost>>,
    /// The plan `/plan` drafted, awaiting `/approve` or `/reject` — shared so
    /// a worker-thread draft reaches the inline `/reject`.
    pub plan: super::plan::SharedPlan,
    /// The commit message `/commit` drafted, awaiting `/commit apply` —
    /// shared for the same worker-vs-inline reason as the plan.
    pub commit: super::gitmsg::SharedCommit,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            overrides: HashMap::new(),
            cancel: Arc::new(AtomicBool::new(false)),
            turns: Arc::new(AtomicU64::new(0)),
            tokens: Arc::new(AtomicU64::new(0)),
            mcp: Arc::new(Mutex::new(crate::mcp::McpHost::from_config())),
            plan: Arc::new(Mutex::new(None)),
            commit: Arc::new(Mutex::new(None)),
        }
    }
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// A worker-thread copy for one task: its own override map (reads only),
    /// the SAME shared counters (turns/tokens) and MCP/plan, but the caller's
    /// per-task `cancel` flag so `/stop #N` reaches exactly this task.
    pub fn snapshot_with_cancel(&self, cancel: Arc<AtomicBool>) -> Self {
        Self {
            overrides: self.overrides.clone(),
            cancel,
            turns: Arc::clone(&self.turns),
            tokens: Arc::clone(&self.tokens),
            mcp: Arc::clone(&self.mcp),
            plan: Arc::clone(&self.plan),
            commit: Arc::clone(&self.commit),
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

    /// A relay broker over `reg` with the env knobs, this session's cancel
    /// flag, and — when the built-in `sys` tools are enabled or MCP servers
    /// are configured — its tools applied; every construct builds its broker
    /// here.
    pub fn broker(&self, reg: Registry) -> Broker {
        let b = Broker::new(reg, max_hops(), call_timeout())
            .with_budget(token_budget())
            .with_cancel_flag(Arc::clone(&self.cancel));
        if !super::systools::enabled() && self.lock_mcp().is_empty() {
            return b;
        }
        b.with_tools(Arc::new(SessionTools(Arc::clone(&self.mcp))))
    }

    /// The shared MCP host, poison-tolerant.
    pub fn lock_mcp(&self) -> std::sync::MutexGuard<'_, crate::mcp::McpHost> {
        self.mcp.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Bridges the engine's [`super::toolcall::ToolRunner`] to the built-in `sys`
/// tools plus the session's shared [`crate::mcp::McpHost`]: one merged TOOLS
/// hint, `sys:` dispatched locally, everything else to MCP.
struct SessionTools(Arc<Mutex<crate::mcp::McpHost>>);

impl super::toolcall::ToolRunner for SessionTools {
    fn hint(&self) -> String {
        let mut tools = if super::systools::enabled() {
            super::systools::tools()
        } else {
            Vec::new()
        };
        tools.extend(self.0.lock().unwrap_or_else(|e| e.into_inner()).tools());
        super::toolcall::hint_for(&tools)
    }

    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        if server == "sys" && super::systools::enabled() {
            return super::systools::call(tool, args);
        }
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .call(server, tool, args)
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
    fn snapshot_with_cancel_uses_the_given_flag() {
        let s = Session::new();
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let snap = s.snapshot_with_cancel(std::sync::Arc::clone(&flag));
        // Tripping the registry-held flag cancels the snapshot's broker/loop.
        flag.store(true, Ordering::Relaxed);
        assert!(
            snap.cancelled(),
            "snapshot observes its own task's cancel flag"
        );
    }

    #[test]
    fn session_tools_hint_lists_sys_tools_with_empty_mcp() {
        use super::super::toolcall::ToolRunner;
        let host = Arc::new(Mutex::new(crate::mcp::McpHost::default()));
        let t = SessionTools(host);
        let h = t.hint();
        // Under `cargo test` no mock/env gate is set, so sys tools are on.
        assert!(h.contains("sys:run"), "{h}");
        assert!(h.contains("sys:read_file"), "{h}");
    }

    #[test]
    fn session_tools_dispatches_sys_locally() {
        use super::super::toolcall::ToolRunner;
        let host = Arc::new(Mutex::new(crate::mcp::McpHost::default()));
        let t = SessionTools(host);
        let r = t
            .call("sys", "run", r#"{"cmd":"echo via-session"}"#)
            .unwrap();
        assert!(r.contains("via-session"), "{r}");
        // Unknown server still falls through to the (empty) MCP host's error.
        let e = t.call("nope", "x", "{}").unwrap_err();
        assert!(e.contains("unknown MCP server"), "{e}");
    }
}

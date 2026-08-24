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
/// Default per-model-call deadline. `sysrun::DEFAULT_TIMEOUT_MS` is pinned
/// below this so a shell command cannot outlive the hop that is waiting on it.
pub(crate) const DEFAULT_CALL_TIMEOUT_MS: u64 = 180_000;
pub(crate) fn call_timeout() -> Duration {
    Duration::from_millis(env_num("CREW_BROKER_TIMEOUT_MS").unwrap_or(DEFAULT_CALL_TIMEOUT_MS))
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
    /// Restored context set by `/resume`, consumed by the next task.
    pub resume: super::sessionlog::SharedResume,
    /// The working tree as of the last automatic checkpoint, so a task that
    /// changed nothing does not write an identical restore point. Shared with
    /// worker snapshots because the checkpoint is taken ON the worker.
    pub last_tree: Arc<Mutex<Option<String>>>,
    /// Whether this session has already mentioned that checkpoints exist.
    /// Once, at the moment the first one is actually taken — an undo nobody
    /// knows about is an undo nobody uses, and a note on every task would be
    /// the noise the silence was protecting against.
    pub announced_ckpt: Arc<AtomicBool>,
    /// Whether this session has already said where to go with a file-change
    /// summary. The LIST is new information after every task and always
    /// reported; "and /diff shows them" is a lesson, and a lesson repeated
    /// after every task is noise — the same rule as `announced_ckpt`.
    pub announced_changes: Arc<AtomicBool>,
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
            resume: Arc::new(Mutex::new(None)),
            last_tree: Arc::new(Mutex::new(None)),
            announced_ckpt: Arc::new(AtomicBool::new(false)),
            announced_changes: Arc::new(AtomicBool::new(false)),
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
            resume: Arc::clone(&self.resume),
            last_tree: Arc::clone(&self.last_tree),
            announced_ckpt: Arc::clone(&self.announced_ckpt),
            announced_changes: Arc::clone(&self.announced_changes),
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
    /// flag, the transcript summarizer (overflow is compacted, not dropped —
    /// keyless/mock keep the clipping), and — when `sys` tools or MCP servers
    /// are on — its tools; every construct builds its broker here.
    pub fn broker(&self, reg: Registry) -> Broker {
        let b = Broker::new(reg, max_hops(), call_timeout())
            .with_budget(token_budget())
            .with_cancel_flag(Arc::clone(&self.cancel))
            .with_summarizer(super::compact::live_summarizer());
        let sys = super::systools::enabled();
        if !sys && self.lock_mcp().is_empty() {
            return b;
        }
        b.with_tools(Arc::new(SessionTools::new(Arc::clone(&self.mcp), sys)))
    }

    /// The shared MCP host, poison-tolerant.
    pub fn lock_mcp(&self) -> std::sync::MutexGuard<'_, crate::mcp::McpHost> {
        self.mcp.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Bridges the engine's [`super::toolcall::ToolRunner`] to the built-in `sys`
/// tools plus the session's shared [`crate::mcp::McpHost`]: one merged TOOLS
/// hint, `sys:` dispatched locally, everything else to MCP.
struct SessionTools {
    mcp: Arc<Mutex<crate::mcp::McpHost>>,
    /// Whether the built-in `sys` surface is on, decided ONCE when the broker
    /// is built rather than re-read on every hint and every call.
    ///
    /// `systools::enabled()` reads two process-global env vars, one of which
    /// (`CREW_BROKER_MOCK_REPLY`) the mock test harness sets and clears while
    /// other tests are running. Two tests here asserted `sys` was on because
    /// "under cargo test no gate is set" — true of a suite running alone, and
    /// false roughly one run in six against a concurrent mocked test, which
    /// is exactly the flake that turned up. It also makes the surface stable
    /// for the life of a session, which is what it was always meant to be.
    sys: bool,
    /// Who this session's tool calls are made on behalf of.
    requester: super::approval::Requester,
    /// The approval gate. Shared, so approvals opened by one call are answerable later.
    gate: Arc<Mutex<super::approval::Gate>>,
    policy: super::approval::Policy,
    /// Where decisions are recorded. `None` in tests that must not touch the user's ledger.
    ledger: Option<super::ledger::Ledger>,
}

impl SessionTools {
    fn new(mcp: Arc<Mutex<crate::mcp::McpHost>>, sys: bool) -> Self {
        Self {
            mcp,
            sys,
            requester: super::approval::Requester::from_env(),
            gate: Arc::new(Mutex::new(super::approval::Gate::new())),
            policy: super::approval::Policy::default(),
            // Never under test. The suite ran once with this unguarded and put twelve
            // records into the real ledger at ~/…/crew/ledger.jsonl — an audit trail that
            // contains a test run's shell calls is worse than no audit trail, because
            // someone reading it later cannot tell which lines were a person.
            ledger: (!cfg!(test)).then(|| super::ledger::Ledger::at(super::ledger::default_path())),
        }
    }

    /// The same runner answering to somebody who is NOT at the keyboard. Unused until a channel
    /// exists to carry the question; it is the reason the gate is wired now rather than later.
    #[cfg(test)]
    fn for_requester(
        mcp: Arc<Mutex<crate::mcp::McpHost>>,
        sys: bool,
        requester: super::approval::Requester,
    ) -> Self {
        Self {
            requester,
            ledger: None,
            ..Self::new(mcp, sys)
        }
    }

    /// Record one line in the action ledger, if there is one. A ledger failure must never fail
    /// the tool call: losing the note is bad, refusing the user's work because a disk is full is
    /// worse, and the failure is visible in the daemon's log either way.
    fn note(&self, r: super::ledger::Record) {
        if let Some(l) = &self.ledger {
            let _ = l.append(&r);
        }
    }
}

impl super::toolcall::ToolRunner for SessionTools {
    fn hint(&self) -> String {
        let mut tools = if self.sys {
            super::systools::tools()
        } else {
            Vec::new()
        };
        tools.extend(self.mcp.lock().unwrap_or_else(|e| e.into_inner()).tools());
        super::toolcall::hint_for(&tools)
    }

    /// Every tool call in the running broker passes through here — `sys` and MCP alike — which
    /// is why the gate is installed at this one point rather than in each tool.
    ///
    /// With today's only requester (a person typing into a pane) the gate always allows, so
    /// nothing about crew's behaviour changes. That is deliberate: the gate belongs in the path
    /// BEFORE a channel can put a non-human behind it, not after.
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        use super::approval::Decision;
        let name = format!("{server}:{tool}");
        let tier = super::tier::tier_of(server, tool);
        let now = super::ledger::now_ms();
        let decision = self.gate.lock().unwrap_or_else(|e| e.into_inner()).decide(
            &name,
            tier,
            &self.requester,
            self.policy,
            now,
        );

        let rec = |decision: &str, note: &str| {
            super::ledger::Record::decided(&name, tier, &self.requester, decision, note)
        };
        match decision {
            Decision::Deny(why) => {
                self.note(rec("deny", &why).with_outcome("denied"));
                return Err(why);
            }
            // Nothing can carry the question yet, so an approval that cannot be asked is a
            // refusal rather than a silent wait. When a channel exists this becomes a real
            // round trip; until then, saying no out loud beats hanging.
            Decision::Ask { id, reply_to } => {
                let why = format!(
                    "{name} needs approval from {reply_to} and no channel can ask yet                      (approval {id})"
                );
                self.note(rec("ask", &why).with_outcome("denied"));
                return Err(why);
            }
            Decision::Allow => {}
        }

        let out = if server == "sys" && self.sys {
            super::systools::call(tool, args)
        } else {
            self.mcp
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .call(server, tool, args)
        };
        // Reads are not recorded: they change nothing, and burying the handful of actions that
        // did change something under thousands of file listings makes the ledger unreadable,
        // which is the same as not having one.
        if tier != super::tier::Tier::Read {
            let outcome = if out.is_ok() { "ran" } else { "failed" };
            let note = out.as_ref().err().map(String::as_str).unwrap_or("");
            self.note(rec("allow", note).with_outcome(outcome));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::approval::Requester;
    use super::super::toolcall::ToolRunner;

    fn host() -> Arc<Mutex<crate::mcp::McpHost>> {
        Arc::new(Mutex::new(crate::mcp::McpHost::default()))
    }

    /// The gate is in the path, and with a person at the keyboard it changes nothing: a read
    /// still reads. (Ledger is None here so the suite never writes the user's audit file.)
    #[test]
    fn a_read_still_runs_for_everyone() {
        for who in [
            Requester::LocalPane,
            Requester::Channel("telegram:me".into()),
            Requester::Trigger("nightly".into()),
        ] {
            let t = SessionTools::for_requester(host(), true, who.clone());
            assert!(
                t.call("sys", "list_dir", "{}").is_ok(),
                "a directory listing changes nothing, so {who:?} may do it"
            );
        }
    }

    /// The behaviour that will matter the moment Telegram lands: a request with no human
    /// watching cannot fire a shell command just because it asked nicely.
    #[test]
    fn a_channel_cannot_run_a_shell_command_without_approval() {
        let t = SessionTools::for_requester(host(), true, Requester::Channel("telegram:me".into()));
        let err = t
            .call("sys", "run", r#"{"cmd": "echo should-not-run"}"#)
            .expect_err("an irreversible call from a channel must not just run");
        assert!(err.contains("needs approval"), "{err}");
        assert!(
            err.contains("telegram:me"),
            "the refusal says who would be asked: {err}"
        );
    }

    /// The 3am case, end to end through the real tool path.
    #[test]
    fn a_trigger_cannot_run_a_shell_command_at_all() {
        let t = SessionTools::for_requester(host(), true, Requester::Trigger("nightly".into()));
        let err = t
            .call("sys", "run", r#"{"cmd": "echo should-not-run"}"#)
            .expect_err("a trigger has nobody to ask");
        assert!(err.contains("cannot be undone"), "{err}");
    }

    /// An MCP server nobody has classified is irreversible by default, so the same refusal
    /// applies to tools crew has never seen.
    #[test]
    fn an_unknown_mcp_tool_from_a_channel_is_gated_too() {
        let t = SessionTools::for_requester(host(), true, Requester::Channel("telegram:me".into()));
        let err = t
            .call("some-server", "send_money", "{}")
            .expect_err("unknown means ask");
        assert!(err.contains("needs approval"), "{err}");
    }

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
        // The verdict is HANDED IN. It used to read the process environment
        // and the comment here said "under `cargo test` no mock/env gate is
        // set, so sys tools are on" — true of this suite running alone, and
        // false whenever a mocked test held CREW_BROKER_MOCK_REPLY, which is
        // about one full run in six.
        let t = SessionTools::new(host, true);
        let h = t.hint();
        assert!(h.contains("sys:run"), "{h}");
        assert!(h.contains("sys:read_file"), "{h}");
    }

    /// …and with the surface off, the hint offers nothing it cannot serve.
    #[test]
    fn session_tools_hint_omits_sys_when_the_surface_is_off() {
        use super::super::toolcall::ToolRunner;
        let host = Arc::new(Mutex::new(crate::mcp::McpHost::default()));
        let h = SessionTools::new(host, false).hint();
        assert!(!h.contains("sys:"), "{h}");
    }

    #[test]
    fn session_tools_dispatches_sys_locally() {
        use super::super::toolcall::ToolRunner;
        let host = Arc::new(Mutex::new(crate::mcp::McpHost::default()));
        let t = SessionTools::new(host, true);
        let r = t
            .call("sys", "run", r#"{"cmd":"echo via-session"}"#)
            .unwrap();
        assert!(r.contains("via-session"), "{r}");
        // Unknown server still falls through to the (empty) MCP host's error.
        let e = t.call("nope", "x", "{}").unwrap_err();
        assert!(e.contains("unknown MCP server"), "{e}");
        // With the surface off, `sys` is not special — it is just another
        // server the empty MCP host has never heard of.
        let off = SessionTools::new(Arc::new(Mutex::new(crate::mcp::McpHost::default())), false);
        let e = off.call("sys", "run", r#"{"cmd":"echo x"}"#).unwrap_err();
        assert!(e.contains("unknown MCP server"), "{e}");
    }
}

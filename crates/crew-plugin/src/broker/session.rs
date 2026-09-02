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
    /// The approval gate, owned by the SESSION rather than by each broker.
    ///
    /// It has to be one gate per pane, not one per construct and certainly not
    /// one per agent: a swarm runs up to `CONCURRENCY` agents at once, and a
    /// gate each would mean being asked to approve the same irreversible tool
    /// four times for one task. Sharing it here is also what lets an approval
    /// opened by one call be answered after another has moved on.
    pub gate: Arc<Mutex<super::approval::Gate>>,
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
            gate: Arc::new(Mutex::new(super::approval::Gate::new())),
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
            gate: Arc::clone(&self.gate),
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
        match self.tools() {
            Some(t) => b.with_tools(t),
            None => b,
        }
    }

    /// This session's tool surface, or `None` when there is nothing to call.
    ///
    /// Built fresh per use rather than cached, because `systools::enabled()`
    /// and the MCP host both change under the session's feet — `mcp.json`
    /// hot-reloads — and a cached `None` from before a server was configured
    /// would be a tool surface that never appears until the pane is reopened.
    /// The GATE is what persists (a session field), not this wrapper.
    ///
    /// The relay reaches this through [`Self::broker`]; the swarm attaches the
    /// same value to its agent factory, so both engines call the same tools
    /// through the same gate into the same ledger.
    pub fn tools(&self) -> Option<Arc<dyn crew_hive::tools::Tools>> {
        self.tools_with_sys(super::systools::enabled())
    }

    /// [`Self::tools`] with the `sys` verdict handed in, so a test can build
    /// the real surface without the process-wide env that decides it.
    pub fn tools_with_sys(&self, sys: bool) -> Option<Arc<dyn crew_hive::tools::Tools>> {
        if !sys && self.lock_mcp().is_empty() && super::integration::load().is_empty() {
            return None;
        }
        Some(Arc::new(SessionTools::new(
            Arc::clone(&self.mcp),
            sys,
            Arc::clone(&self.gate),
        )))
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
    /// The approval gate, shared with the session (and so with every other
    /// agent and construct in this pane) — see [`Session::gate`].
    gate: Arc<Mutex<super::approval::Gate>>,
    policy: super::approval::Policy,
    /// Where decisions are recorded. `None` in tests that must not touch the user's ledger.
    ledger: Option<super::ledger::Ledger>,
    /// Manifest-defined HTTP integrations, read when this surface is built. `SessionTools` is
    /// built fresh per hop, so dropping a file in `~/.config/crew/integrations/` takes effect on
    /// the next task with no restart — the same hot-reload skills and `mcp.json` have.
    integrations: Vec<super::integration::Integration>,
}

impl SessionTools {
    fn new(
        mcp: Arc<Mutex<crate::mcp::McpHost>>,
        sys: bool,
        gate: Arc<Mutex<super::approval::Gate>>,
    ) -> Self {
        Self {
            mcp,
            sys,
            requester: super::approval::Requester::from_env(),
            gate,
            policy: super::approval::Policy::default(),
            // Never under test. The suite ran once with this unguarded and put twelve
            // records into the real ledger at ~/…/crew/ledger.jsonl — an audit trail that
            // contains a test run's shell calls is worse than no audit trail, because
            // someone reading it later cannot tell which lines were a person.
            ledger: (!cfg!(test)).then(|| super::ledger::Ledger::at(super::ledger::default_path())),
            integrations: super::integration::load(),
        }
    }

    /// A runner with a gate of its own, for tests that do not care which
    /// gate — the session-shared one is not reachable from here.
    #[cfg(test)]
    fn for_test(mcp: Arc<Mutex<crate::mcp::McpHost>>, sys: bool) -> Self {
        Self::new(mcp, sys, Arc::new(Mutex::new(super::approval::Gate::new())))
    }

    /// [`Self::for_test`] with integrations handed in rather than read from disk. The
    /// discovery path is tested in `integration::tests`; wiring it through `CREW_PROJECT_DIR`
    /// here would put a process-global env var in a suite that runs in parallel — the exact
    /// flake the `sys` field's comment above records.
    #[cfg(test)]
    fn with_integrations(sys: bool, integrations: Vec<super::integration::Integration>) -> Self {
        Self {
            integrations,
            ..Self::for_test(Arc::new(Mutex::new(crate::mcp::McpHost::default())), sys)
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
            ..Self::for_test(mcp, sys)
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

impl SessionTools {
    /// Every tool this session can reach: crew's own, then every connected MCP server's.
    fn catalog(&self) -> Vec<crate::mcp::McpTool> {
        let mut tools = if self.sys {
            super::systools::tools()
        } else {
            Vec::new()
        };
        tools.extend(self.mcp.lock().unwrap_or_else(|e| e.into_inner()).tools());
        tools.extend(super::integration::tools_of(&self.integrations));
        tools
    }

    /// The integration owning `server`, if one does.
    fn integration(&self, server: &str) -> Option<&super::integration::Integration> {
        self.integrations.iter().find(|i| i.name == server)
    }

    /// The reversibility class of one tool. An integration's manifest says its own, defaulting
    /// to irreversible; everything else is classified where it always was.
    fn tier_for(&self, server: &str, tool: &str) -> super::tier::Tier {
        match self.integration(server) {
            Some(i) => i.tier_of(tool),
            None => super::tier::tier_of(server, tool),
        }
    }

    /// Run one manifest-defined tool: build the request, send it, hand back the body.
    fn call_integration(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        let int = self
            .integration(server)
            .ok_or_else(|| format!("no integration named {server}"))?;
        let spec = int
            .tools
            .iter()
            .find(|t| t.name == tool)
            .ok_or_else(|| format!("{server} has no tool {tool}"))?;
        super::integration::run::send(super::integration::request::build(int, spec, args)?)
    }
}

impl super::toolcall::ToolRunner for SessionTools {
    fn hint(&self) -> String {
        super::toolcall::hint_for(&self.catalog())
    }

    /// One line per integration and per MCP server, for the planner.
    fn capabilities(&self) -> Vec<String> {
        let mcp = self.mcp.lock().unwrap_or_else(|e| e.into_inner()).tools();
        super::capabilities::lines(&self.integrations, &mcp)
    }

    /// The tools worth naming for THIS task, plus a count of what was left out.
    ///
    /// Below [`toolpick::BUDGET`] this is exactly [`Self::hint`]; above it, the task decides.
    /// See `toolpick` for why the alternative — every tool on every hop — fails at forty.
    fn hint_for(&self, task: &str) -> String {
        let (picked, left_out) =
            super::toolpick::pick(self.catalog(), task, super::toolpick::BUDGET);
        let mut hint = super::toolcall::hint_for(&picked);
        if !hint.is_empty() {
            hint.push_str(&super::toolpick::omitted_note(left_out));
        }
        hint
    }

    /// Structured definitions for native tool-use: the same merged surface
    /// [`Self::hint`] describes in prose, with each tool's real JSON Schema.
    ///
    /// Both must stay in step, because the PROVIDER decides which of the two a
    /// given run uses — a tool present in one and not the other is a tool that
    /// appears and disappears depending on which model is serving.
    fn specs(&self) -> Vec<crew_hive::tools::ToolSpec> {
        specs_of(self.catalog())
    }

    /// The same selection [`Self::hint_for`] makes, in the native shape. The two must agree:
    /// the provider decides which path runs, and a tool in one and not the other is a tool that
    /// appears and disappears depending on which model is serving.
    fn specs_for(&self, task: &str) -> Vec<crew_hive::tools::ToolSpec> {
        specs_of(super::toolpick::pick(self.catalog(), task, super::toolpick::BUDGET).0)
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
        let tier = self.tier_for(server, tool);
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

        let out = if server == "sys" && tool == "find_tools" && self.sys {
            // Answered HERE rather than in `systools`, because the thing being searched is the
            // session's own catalog and nothing below this point has it. It is the door that
            // makes retrieval safe: a tool the picker left out is one question away.
            Ok(super::toolpick::search(
                &self.catalog(),
                &search_query(args),
                super::toolpick::BUDGET,
            ))
        } else if server == "sys" && self.sys {
            super::systools::call(tool, args)
        } else if self.integration(server).is_some() {
            self.call_integration(server, tool, args)
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

/// The `q` of a `sys:find_tools` call. A malformed or missing argument searches for nothing,
/// which lists nothing and says how many tools there are — more useful than an error, because
/// the model's next move is to search again with a word in it.
fn search_query(args: &str) -> String {
    serde_json::from_str::<serde_json::Value>(args)
        .ok()
        .and_then(|v| v.get("q")?.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Tool descriptors in the shape a provider is handed.
fn specs_of(tools: Vec<crate::mcp::McpTool>) -> Vec<crew_hive::tools::ToolSpec> {
    tools
        .into_iter()
        .map(|t| crew_hive::tools::ToolSpec {
            server: t.server,
            tool: t.name,
            description: t.description,
            input_schema: t.input_schema,
        })
        .collect()
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;

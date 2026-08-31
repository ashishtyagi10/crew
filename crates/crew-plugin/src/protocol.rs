use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginCommand {
    Hello {
        v: u32,
    },
    Subscribe {
        channel: String,
    },
    Send {
        channel: String,
        text: String,
    },
    /// Answer an approval the broker is blocked on. `id` comes from the
    /// [`PluginEvent::Approval`] that opened it.
    Approve {
        id: String,
        granted: bool,
    },
}

/// One agent in a plugin's roster: its address name, a short capability role,
/// and the model it runs on (empty when unknown, e.g. an external CLI).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInfo {
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginEvent {
    Ready {
        v: u32,
        provider: String,
        channels: Vec<String>,
    },
    /// The agents this plugin can route to (sent once after `Ready`), so the
    /// host can show a roster with model badges.
    Roster {
        agents: Vec<AgentInfo>,
    },
    /// A live status change: `agent` entered `state` (`"thinking"` while being
    /// called; `"idle"` with an empty agent when the turn ends). `from` names
    /// who handed the agent its work (`"user"`, a peer agent, …; may be empty),
    /// so the host can draw the live interaction, not just a busy flag.
    Activity {
        agent: String,
        state: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        from: String,
    },
    /// End-of-turn cost: agent exchanges made and approximate tokens spent.
    /// Feeds the host's running token meter. When `agent` is non-empty the
    /// event is one agent's reply stat instead — `agent` spent `ms` (and
    /// `tokens`, when the backend reports real usage) on one reply, streamed
    /// live as the hop lands — feeding the host's per-agent totals.
    Stats {
        exchanges: u32,
        tokens: u64,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        agent: String,
        #[serde(default)]
        ms: u64,
        /// The reply's real prompt size in tokens — the agent's live context
        /// fill — when the backend reports usage; 0 = unknown.
        #[serde(default)]
        ctx: u64,
        /// Prompt/completion token split for the same usage `tokens` reports,
        /// and the broker-computed cost in micro-USD (0 = unpriced model).
        /// All serde-defaulted so old payloads still decode.
        #[serde(default)]
        tok_in: u64,
        #[serde(default)]
        tok_out: u64,
        #[serde(default)]
        cost_microusd: u64,
    },
    /// A drafted plan is waiting for a decision (`pending: true`), or that
    /// decision has been made (`false`).
    ///
    /// `/plan` drafts and stops; something has to say yes. That used to be two
    /// more constructs to know — `/approve` and `/reject` — for a question with
    /// exactly two answers, which is a keypress, not a vocabulary. The host
    /// shows the pending state and sends the construct itself.
    Plan {
        pending: bool,
    },
    /// A background task started (`running: true`) or ended (`false`).
    ///
    /// The broker runs several tasks at once, each on its own worker thread
    /// with a monotonic id, and until now the only way to learn what was in
    /// flight was to ASK — `/tasks`. Nothing could be shown, because nothing
    /// was ever sent: the registry is reaped lazily on the next command, so a
    /// task finishing was not an observable moment anywhere.
    ///
    /// It is now. The start is emitted by the stdin loop as it spawns; the end
    /// by the worker itself as it exits, which is the only place that knows.
    /// `label` rides the start only — the host already has it by the time the
    /// task ends.
    Task {
        id: u64,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        label: String,
        running: bool,
    },
    /// Mid-reply progress: `agent` has produced roughly `tokens` output
    /// tokens so far in its in-flight reply. Advisory — the end-of-hop
    /// `Stats` stays authoritative and reconciles any estimate drift.
    StatsTick {
        agent: String,
        tokens: u64,
    },
    /// Mid-reply text: `agent` produced `text` since the previous Delta of
    /// this hop. ADVISORY, exactly like `StatsTick` — the end-of-hop
    /// `Message` carries the full normalized reply and REPLACES anything
    /// streamed here, so a dropped or coalesced fragment can never corrupt
    /// the transcript.
    Delta {
        agent: String,
        text: String,
    },
    /// An irreversible tool call is waiting for a human. The broker is BLOCKED on
    /// this — the agent's tool call does not return until the answer arrives or
    /// the approval lapses — so a host that receives one must either carry it to
    /// whoever can answer or leave it to time out.
    Approval {
        id: String,
        /// `server:name` of the tool.
        tool: String,
        /// Reversibility class, as a word.
        tier: String,
        /// The address the question should be put to.
        reply_to: String,
        /// A human-readable line describing what is about to happen.
        question: String,
    },
    Message {
        channel: String,
        sender: String,
        text: String,
        /// Unix-epoch milliseconds when the message was produced ("" = unknown).
        ts: String,
        /// Optional per-message metadata for the host's log line (e.g. the
        /// reply's latency, `"4.2s"`). Absent on the wire when empty.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        meta: String,
    },
    /// A swarm plan landed: the full task list, so the host can open/refresh
    /// the companion graph pane. Sent once per swarm run, before execution.
    HivePlan {
        tasks: Vec<crew_hive::TaskSpec>,
    },
    /// One raw swarm telemetry event, forwarded verbatim for the host's
    /// companion graph pane. Chat-facing translations are sent separately.
    Hive {
        event: crew_hive::HiveEvent,
    },
    /// A one-line status note for the host's activity LOG (not the chat
    /// transcript): background lifecycle the pane user would otherwise never
    /// see — an MCP server connecting, a connection dying. `error: true`
    /// renders in the host's attention color. Older hosts skip the unknown
    /// tag (see `unknown_event_type_fails_to_parse_so_the_host_can_skip_it`).
    Status {
        #[serde(default)]
        error: bool,
        message: String,
    },
    Error {
        message: String,
    },
    SpawnPane {
        command: String,
        args: Vec<String>,
        label: String,
    },
    SendPane {
        label: String,
        text: String,
    },
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;

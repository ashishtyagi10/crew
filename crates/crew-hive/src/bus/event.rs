use crate::graph::{TaskId, TaskState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AgentId(pub u64);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum HiveEvent {
    TaskStateChanged {
        task: TaskId,
        state: TaskState,
    },
    AgentSpawned {
        agent: AgentId,
        task: TaskId,
    },
    TokenDelta {
        agent: AgentId,
        input: u32,
        output: u32,
    },
    CostDelta {
        agent: AgentId,
        micros_usd: u64,
    },
    /// One streamed fragment of an agent's in-flight reply, published as it
    /// arrives from the provider. ADVISORY: the `OutputChunk` published when
    /// the agent finishes carries the COMPLETE output and is what the
    /// transcript keeps, so a subscriber that misses deltas — or ignores them
    /// entirely — loses liveness, never content.
    OutputDelta {
        agent: AgentId,
        text: String,
    },
    /// One streamed fragment of an agent's REASONING, as the provider showed
    /// it (`Chunk::Thought`). Its own event, not an `OutputDelta`: the
    /// working is not the reply, and a subscriber that appended it to the
    /// reply card would read the model's scratch back to it as its own
    /// words on the next turn. Unlike `OutputDelta` there is no settled twin
    /// — the deltas are the only carrier — so `apiagent::chunks` publishes a
    /// non-streamed completion's whole `thought` as one of these.
    ThoughtDelta {
        agent: AgentId,
        text: String,
    },
    OutputChunk {
        agent: AgentId,
        text: String,
    },
    /// An agent asked for a tool, published BEFORE it runs. Emitted as its own
    /// event rather than folded into the agent's output text because a tool
    /// call is a different kind of thing from a reply: it is an action against
    /// the world, it is what the ledger records, and a pane that shows it as
    /// prose cannot style it, count it, or let a person stop it.
    ToolCall {
        agent: AgentId,
        /// `server:tool`.
        label: String,
        args: String,
    },
    /// The outcome of the matching [`HiveEvent::ToolCall`]. `ok` is false for
    /// a refusal or an error — including one the approval gate returned, which
    /// is a normal outcome and not a task failure.
    ToolResult {
        agent: AgentId,
        label: String,
        ok: bool,
        text: String,
        /// How long the call took, in milliseconds.
        ///
        /// A tool is the one part of a run whose duration is not the model's:
        /// `sys:run` alone may sit for two minutes on its deadline, and an
        /// agent waiting on it produces no text at all. Without this the pane
        /// cannot tell a slow tool from a hung one, which is the question a
        /// person actually has while watching.
        #[serde(default)]
        ms: u64,
    },
    Failed {
        agent: AgentId,
        error: String,
    },
    /// Something the run reached for came in — a skill playbook was spliced
    /// into a prompt, an MCP server answered its handshake, a language server
    /// started. Not a tool call (nothing ran, nothing to approve) yet the same
    /// kind of news: an action against the world that shapes the reply and
    /// that the pane could not otherwise show. Without it a skill rewrites
    /// the prompt in silence and a 13-second `npx` download reads as a hang.
    ///
    /// `agent` is a NAME, not an [`AgentId`]: the relay engine has no hive
    /// ids, and an MCP connect happens under whoever's call forced it — so
    /// the empty string means "whoever is active" and the pane decides.
    Loaded {
        #[serde(default)]
        agent: String,
        /// `"skill"`, `"mcp"` or `"lsp"`.
        #[serde(default)]
        kind: String,
        /// The skill's name, the server's name, the language server command.
        #[serde(default)]
        name: String,
        /// One short human line: `applied · <one-liner>`,
        /// `connected · 12 tools: a, b, …`, `<lang> · <root dir>`.
        #[serde(default)]
        detail: String,
    },
}

//! The tool surface every crew agent shares.
//!
//! This lives in crew-hive, not in the broker, because BOTH engines need it
//! and only one of them had it. The `@agent` relay has had tools since the MCP
//! host landed; the parallel swarm — the default `/crew` path — had none, so
//! crew owned a parallel engine that could not reach the world and a
//! world-reaching engine that could not run in parallel. The trait and the
//! parser live at the bottom of the dependency graph so one implementation
//! serves both, rather than the swarm growing a second dialect of `@tool`.
//!
//! The call convention is a TEXT one: an agent ends its reply with
//! `@tool <server>:<tool> {"arg": …}`. That is a stopgap and it is documented
//! as one — [`crate::provider::CompletionRequest`] is `{model, system, prompt,
//! max_tokens}` and cannot express a native tool-use turn, so until it grows
//! messages and schemas this is the convention crew has. It has the virtue of
//! already being proven in the relay against real models.

mod around;
pub mod budget;
mod catalog;
pub mod exchanges;
mod jsondepth;
pub mod near;
mod parse;
mod said;
pub mod seen;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod failed_tests;

/// One tool as CREW names it, before any provider gets to see it.
///
/// crew's identity for a tool is `server:tool`, which no provider accepts as a
/// function name — Anthropic and the OpenAI shape both require
/// `[a-zA-Z0-9_-]{1,64}`. Keeping crew's spelling here and doing the encoding
/// in exactly one place ([`ToolCatalog`]) is what stops `server:tool`,
/// `server__tool` and `server-tool` all existing at once.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolSpec {
    pub server: String,
    pub tool: String,
    pub description: String,
    /// JSON Schema for the arguments. `{"type":"object"}` when the source
    /// declared nothing — never `null`, which providers reject.
    pub input_schema: serde_json::Value,
}

impl ToolSpec {
    /// `server:tool` — crew's spelling, for prompts, events and the ledger.
    pub fn label(&self) -> String {
        format!("{}:{}", self.server, self.tool)
    }
}

pub use catalog::ToolCatalog;
pub use jsondepth::JsonDepth;
pub use parse::{parse_tool_call, split_tool_call, split_tool_calls};
pub use said::said;

/// Executes tool calls on behalf of an agent.
///
/// SYNCHRONOUS on purpose: the implementation on the other side of this trait
/// (the broker's MCP host and `sys` surface) is blocking, and pretending
/// otherwise would mean an async wrapper around a blocking call — the worst of
/// both. Callers inside a future MUST therefore push it to a blocking pool;
/// see [`crate::apiagent`], where a tool call that blocked the scheduler's
/// current-thread runtime would stall every other agent in the swarm and the
/// event drain with them.
pub trait Tools: Send + Sync {
    /// The prompt section advertising what can be called (empty = nothing).
    fn hint(&self) -> String;
    /// Run one tool. `Err` is shown to the agent, never propagated as a task
    /// failure: a tool that refuses is information the agent can act on.
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String>;

    /// Whether an `Ok` from [`call`](Tools::call) is a failed call all the same.
    ///
    /// A shell command that ran and exited 101 comes back `Ok`, because its
    /// output is the answer — and was shown as a green card and sent to the
    /// provider with `is_error: false`, so the model read a broken build as
    /// data it could trust. Every engine asks here before calling a result
    /// ok; the surface that knows its own convention answers. Defaults to no.
    fn failed(&self, _server: &str, _tool: &str, _output: &str) -> bool {
        false
    }

    /// Whether `server:tool` only LOOKS, so the same call made again with
    /// nothing written in between would answer what it answered the first time.
    ///
    /// A routed reply spent its turn re-reading one file, and every repeat ran,
    /// put its whole result into the prompt again and cost a round. The loops
    /// now answer such a repeat with a pointer to the first result
    /// ([`seen::Seen`]), but only the surface knows which of its tools change
    /// nothing. Defaults to no: a tool nobody classified is never skipped.
    fn repeatable(&self, _server: &str, _tool: &str) -> bool {
        false
    }

    /// Structured definitions, for providers that speak native tool-use.
    ///
    /// Defaults to EMPTY, which means "fall back to the `@tool` text
    /// convention in [`hint`]". A `Tools` implementation that has schemas is
    /// expected to return them here AND keep `hint` working, because the
    /// provider decides which path runs, not the tool surface.
    ///
    /// [`hint`]: Tools::hint
    fn specs(&self) -> Vec<ToolSpec> {
        Vec::new()
    }

    /// What an agent can REACH, one line each, for the PLANNER: an
    /// integration and what it is for, an MCP server and what it holds.
    ///
    /// The planner never sees a tool hint — that is per task, per hop, and
    /// the planner runs once before there are tasks — so it could not know a
    /// goal was reachable, and could not route a task to the specialist who
    /// would hold the right integration. Coarse on purpose: a name and a
    /// sentence per source, not a tool list. Defaults to nothing.
    fn capabilities(&self) -> Vec<String> {
        Vec::new()
    }

    /// [`hint`] for one task in particular — the RETRIEVAL seam.
    ///
    /// Defaults to [`hint`], so an implementation with a handful of tools needs nothing. An
    /// implementation with two hundred is expected to show the ones the task could plausibly
    /// want, say how many it left out, and leave a way to reach the rest: the prompt is
    /// O(all tools) per hop per agent otherwise, and selection accuracy collapses long before
    /// the token bill does.
    ///
    /// [`hint`]: Tools::hint
    fn hint_for(&self, _task: &str) -> String {
        self.hint()
    }

    /// [`specs`] for one task in particular, under the same contract as [`hint_for`].
    ///
    /// The two MUST select alike: the provider decides which path runs, and a tool present in
    /// one and absent from the other is a tool that appears and disappears depending on which
    /// model is serving.
    ///
    /// [`specs`]: Tools::specs
    fn specs_for(&self, _task: &str) -> Vec<ToolSpec> {
        self.specs()
    }

    /// One line of prose for the NATIVE path about the tools that are NOT on
    /// the wire: how many [`specs_for`] left out, and what to call to reach
    /// them. `None` (the default) means every tool is on the wire, and the
    /// prompt is left byte-identical.
    ///
    /// The native path sends no tools prose on purpose — the schemas are on
    /// the wire, and naming the `@tool` convention beside them invites a model
    /// to use both. But a model shown twenty-four of forty tools and told
    /// nothing cannot know the other sixteen exist, let alone that one of the
    /// twenty-four searches them. This is the one sentence that crosses.
    ///
    /// [`specs_for`]: Tools::specs_for
    fn note_for(&self, _task: &str) -> Option<String> {
        None
    }
}

/// Tool rounds per TASK, which the swarm pools over the run ([`budget`]) and
/// the relay spends per hop.
///
/// Sized for the relay, where a hop is one question. It is deliberately the
/// same number here so the two engines behave alike, and it is deliberately
/// small: every round is a whole extra model call, billed and waited on.
pub const MAX_TOOL_ROUNDS: u32 = 4;

/// A parsed `@tool server:tool {json}` directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    pub server: String,
    pub tool: String,
    pub args: String,
}

impl ToolCall {
    /// `server:tool`, the form used in prompts, events and the ledger.
    pub fn label(&self) -> String {
        format!("{}:{}", self.server, self.tool)
    }
}

/// The task text an agent sees: the body, plus the tools section when there
/// is one. An empty hint must leave the body BYTE-IDENTICAL — a swarm with no
/// tools configured has to behave exactly as it did before this module.
pub fn augment(body: &str, hint: &str) -> String {
    if hint.is_empty() {
        return body.to_string();
    }
    format!("{body}\n\n{hint}")
}

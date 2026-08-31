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

#[cfg(test)]
mod tests;

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
}

/// Most tool rounds one agent may take within a single task.
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

/// Read a tool directive off the reply's last non-empty line, tolerating the
/// markdown wrappers models add unbidden (`**@tool …**`, a lone backtick).
/// `None` = the agent answered instead of calling something.
pub fn parse_tool_call(reply: &str) -> Option<ToolCall> {
    let last = reply.lines().rev().find(|l| !l.trim().is_empty())?.trim();
    let last = last.trim_start_matches(['*', '`', '_', ' ']);
    if !last.to_ascii_lowercase().starts_with("@tool ") {
        return None;
    }
    let rest = last[6..].trim();
    let (target, args) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let target = target.trim_matches(['`', '*', '_']);
    let (server, tool) = target.split_once(':')?;
    (!server.is_empty() && !tool.is_empty()).then(|| ToolCall {
        server: server.to_string(),
        tool: tool.to_string(),
        args: args.trim().trim_matches('`').to_string(),
    })
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

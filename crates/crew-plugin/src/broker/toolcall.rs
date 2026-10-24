//! Mid-relay tool calls: the tool surface a relay turn is given, and the
//! TOOLS section its frame advertises. An agent calls a tool by ending its
//! reply with `@tool <server>:<tool> {"arg": …}` (several on consecutive last
//! lines), or natively when its provider speaks tool-use; the turn itself —
//! running the calls, showing the agent the results, the round budget — is
//! the harness's (`hopturn`), the same loop the swarm runs.
use std::sync::Arc;

use super::route::clip;
use super::Broker;
use crate::mcp::McpTool;

/// Executes tool calls. Implemented over the session's shared
/// [`crate::mcp::McpHost`]; tests use fakes.
///
/// THE TRAIT LIVES IN CREW-HIVE, and this is an alias for it. The swarm needs
/// the same surface the relay has, crew-hive sits below both, and two
/// definitions of "a thing that runs a tool" would have meant two `@tool`
/// dialects drifting apart — the relay's and the swarm's — with the MCP host
/// implementing each separately.
pub use crew_hive::tools::Tools as ToolRunner;

/// Most tool rounds one agent may take within a single hop, a round being
/// one reply however many calls it makes. Shared with the swarm so both
/// engines stop at the same number.
pub(crate) use crew_hive::tools::MAX_TOOL_ROUNDS;

/// The TOOLS prompt section for `tools` (empty when there are none).
pub(crate) fn hint_for(tools: &[McpTool]) -> String {
    if tools.is_empty() {
        return String::new();
    }
    let lines: Vec<String> = tools
        .iter()
        .map(|t| {
            // FIRST LINE: a server's paragraphs would break the list. Clipped,
            // but crew's own `sys:` tools whole: at 100, sys:edit lost `edits`,
            // `all` and `line` (3 of 3 first edits refused for want of `line`).
            let one = t.description.lines().next().unwrap_or("");
            let cap = if t.server == "sys" { usize::MAX } else { 100 };
            format!("- {}:{} \u{2014} {}", t.server, t.name, clip(one, cap))
        })
        .collect();
    // LOOK FIRST turns a tool list into grounded answers: offered as merely
    // "optional", a model asked what a repo crate does answered from its
    // priors (a web framework that does not exist), sys:read_file right there.
    format!(
        "TOOLS: to call one, make the FINAL line of your reply exactly\n\
         `@tool <server>:<tool> {{\"arg\": \u{2026}}}` (JSON arguments) \u{2014} the \
         result is sent back to you before you answer. Several reads may go on \
         consecutive last lines, one call per line, and run together.\n\
         LOOK FIRST: when the task is about this project \u{2014} its code, files, \
         build or behaviour \u{2014} find the relevant files (sys:grep, sys:glob) and \
         read them before you answer. Never describe code you have not read, and \
         never guess a path.\nAvailable tools:\n{}",
        lines.join("\n")
    )
}

/// The tools section for a task whose body is `body` (empty when no tools
/// are attached). Kept apart from the body so [`super::route::frame`] can
/// clip the one and never the other.
pub(crate) fn hint(body: &str, tools: Option<&dyn ToolRunner>) -> String {
    tools.map(|t| t.hint_for(body)).unwrap_or_default()
}

impl Broker {
    /// Let agents call tools mid-relay through `runner`.
    pub fn with_tools(mut self, runner: Arc<dyn ToolRunner>) -> Self {
        self.tools = Some(runner);
        self
    }
}

#[cfg(test)]
#[path = "toolcall_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "samecall_tests.rs"]
mod samecall_tests;

#[cfg(test)]
#[path = "shrinkold_tests.rs"]
mod shrinkold_tests;

#[cfg(test)]
#[path = "toolstop_tests.rs"]
mod toolstop_tests;

#[cfg(test)]
#[path = "todorelay_tests.rs"]
mod todorelay_tests;

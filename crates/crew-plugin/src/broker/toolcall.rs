//! Mid-relay tool calls. When a [`ToolRunner`] is attached, every agent's
//! task advertises the available tools; an agent calls one by ending its
//! reply with `@tool <server>:<tool> {"arg": …}`. The engine executes the
//! call and re-dials the same agent with the result — up to
//! [`MAX_TOOL_ROUNDS`] times per hop — before routing resumes. Every call
//! and result is logged as a hop, so tool use is visible in the pane.
use std::sync::Arc;

use super::adapter::Adapter;
use super::hop::{back, Hop, HopKind, RunStats};
use super::route::clip;
use super::toolclip::clip_result;
use super::{Broker, Envelope};
use crate::mcp::McpTool;

/// Executes tool calls for the engine. Implemented over the session's shared
/// [`crate::mcp::McpHost`]; tests use fakes.
pub trait ToolRunner: Send + Sync {
    /// The prompt section advertising available tools (empty = none).
    fn hint(&self) -> String;
    /// Run one tool; both sides of the result flow back to the agent.
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String>;
}

/// Most tool rounds one agent may take within a single hop.
pub(crate) const MAX_TOOL_ROUNDS: u32 = 4;

/// The TOOLS prompt section for `tools` (empty when there are none).
pub(crate) fn hint_for(tools: &[McpTool]) -> String {
    if tools.is_empty() {
        return String::new();
    }
    let lines: Vec<String> = tools
        .iter()
        .map(|t| {
            format!(
                "- {}:{} \u{2014} {}",
                t.server,
                t.name,
                clip(&t.description, 100)
            )
        })
        .collect();
    format!(
        "TOOLS (optional): to call one, make the FINAL line of your reply exactly\n\
         `@tool <server>:<tool> {{\"arg\": \u{2026}}}` (JSON arguments) \u{2014} the \
         result is sent back to you before you answer.\nAvailable tools:\n{}",
        lines.join("\n")
    )
}

/// The task text an agent sees: the body, plus the tools section when tools
/// are attached.
pub(crate) fn augment(body: &str, tools: Option<&dyn ToolRunner>) -> String {
    match tools.map(|t| t.hint()) {
        Some(h) if !h.is_empty() => format!("{body}\n\n{h}"),
        _ => body.to_string(),
    }
}

/// A parsed `@tool server:tool {json}` directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolCall {
    pub server: String,
    pub tool: String,
    pub args: String,
}

/// Read a tool directive off the reply's last non-empty line (tolerating the
/// same markdown wrappers as routing directives). `None` = no tool call.
pub(crate) fn parse_tool_call(reply: &str) -> Option<ToolCall> {
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

impl Broker {
    /// Let agents call tools mid-relay through `runner`.
    pub fn with_tools(mut self, runner: Arc<dyn ToolRunner>) -> Self {
        self.tools = Some(runner);
        self
    }

    /// Resolve any tool directives in `reply`: run the tool, show the agent
    /// the result, and take its next reply — until it answers without a tool
    /// call or the round cap trips. Returns the reply routing should parse.
    pub(crate) fn run_tools(
        &self,
        agent: &dyn Adapter,
        base_prompt: &str,
        mut reply: String,
        stats: &mut RunStats,
        env: &Envelope,
        sink: &mut dyn FnMut(Hop),
    ) -> String {
        let Some(runner) = self.tools.as_deref() else {
            return reply;
        };
        let mut exchanges: Vec<String> = Vec::new();
        for _ in 0..MAX_TOOL_ROUNDS {
            let Some(call) = parse_tool_call(&reply) else {
                return reply;
            };
            let label = format!("{}:{}", call.server, call.tool);
            sink(Hop {
                from: env.to.clone(),
                to: label.clone(),
                hop: env.hop,
                kind: HopKind::Reply,
                text: format!("[tool] {label} {}", clip(&call.args, 200)),
                usage: Default::default(),
            });
            let text = match runner.call(&call.server, &call.tool, &call.args) {
                Ok(t) if t.is_empty() => "(empty result)".to_string(),
                Ok(t) => t,
                Err(e) => format!("ERROR: {e}"),
            };
            stats.approx_tokens += text.len() / 4;
            sink(Hop {
                from: label.clone(),
                to: env.to.clone(),
                hop: env.hop,
                kind: HopKind::Reply,
                text: clip(&text, 400),
                usage: Default::default(),
            });
            exchanges.push(format!(
                "CALLED {label} {}\nRESULT:\n{}",
                call.args,
                clip_result(&text, 6000)
            ));
            let follow = format!(
                "{base_prompt}\n\nTOOL EXCHANGES THIS TURN:\n{}\n\nContinue the task \
                 using these results. You may call another tool, or answer and end \
                 with your routing line (`@next <agent>` or `@done`).",
                exchanges.join("\n\n")
            );
            sink(Hop {
                from: label,
                to: env.to.clone(),
                hop: env.hop,
                kind: HopKind::Dialing,
                text: String::new(),
                usage: Default::default(),
            });
            match agent.call(&follow, self.timeout) {
                Ok(r) if !r.trim().is_empty() => {
                    stats.exchanges += 1;
                    stats.approx_tokens += (follow.len() + r.len()) / 4;
                    reply = r;
                }
                Ok(_) => {
                    sink(back(
                        env,
                        HopKind::Error,
                        "empty reply after tool call".into(),
                    ));
                    return reply;
                }
                Err(e) => {
                    sink(back(env, HopKind::Error, e));
                    return reply;
                }
            }
        }
        reply
    }
}

#[cfg(test)]
#[path = "toolcall_tests.rs"]
mod tests;

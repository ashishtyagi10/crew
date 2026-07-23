//! Mid-relay tool calls. When a [`ToolRunner`] is attached, every agent's
//! task advertises the available tools; an agent calls one by ending its
//! reply with `@tool <server>:<tool> {"arg": …}`. The engine executes the
//! call and re-dials the same agent with the result — up to
//! [`MAX_TOOL_ROUNDS`] times per hop — before routing resumes. Every call
//! and result is logged as a hop, so tool use is visible in the pane.
use std::sync::Arc;

use super::adapter::{Adapter, Usage};
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
    ///
    /// `usage` is the surrounding hop's usage (the primary dial's, or the
    /// repair dial's when it ran) — each follow-up dial overwrites it with
    /// its own real usage, the same "latest wins" rule the primary dial uses
    /// for its own repair call, so the hop's reply-stat and context-fill
    /// stay accurate through tool rounds.
    ///
    /// `on_tokens` is the SAME hop ticker the primary dial was given (built
    /// once, by `crate::broker::tick::hop_ticker`, at the engine call site)
    /// — reused rather than rebuilt per follow-up so the per-agent 150ms
    /// gate spans the whole hop, primary dial plus every follow-up. But that
    /// gate only ever emits on GROWTH, and each `call_with_usage_ticked`
    /// restarts its own chars/4 estimate at 0 — so a follow-up dial can't
    /// just report its own running total, or a short follow-up would never
    /// climb past a long primary dial's last tick and would tick zero times
    /// for its whole duration. Each follow-up dial instead reports an
    /// OFFSET estimate: its own total plus `tick_base`, the running sum of
    /// every prior dial's final chars/4 in this hop (primary dial included).
    /// That keeps every follow-up's reported value monotonically past
    /// wherever the hop's shared gate left off, so it survives the growth
    /// check instead of being swallowed by it.
    #[allow(clippy::too_many_arguments)] // engine-loop plumbing, one call site
    pub(crate) fn run_tools(
        &self,
        agent: &dyn Adapter,
        base_prompt: &str,
        mut reply: String,
        stats: &mut RunStats,
        usage: &mut Usage,
        env: &Envelope,
        on_tokens: &Arc<dyn Fn(u64) + Send + Sync>,
        sink: &mut dyn FnMut(Hop),
    ) -> String {
        let Some(runner) = self.tools.as_deref() else {
            return reply;
        };
        // The hop's running chars/4 estimate so far: the primary dial's own
        // reply, since `reply` at entry is what it produced. Follow-up dials
        // offset by this (and each other's) so the shared ticker's growth
        // gate never swallows a short follow-up after a long primary reply.
        let mut tick_base: u64 = (reply.chars().count() as u64) / 4;
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
            let base = tick_base;
            let ticked: Arc<dyn Fn(u64) + Send + Sync> = {
                let on = on_tokens.clone();
                Arc::new(move |t| on(base + t))
            };
            match agent.call_with_usage_ticked(&follow, self.timeout, ticked) {
                Ok((r, u)) if !r.trim().is_empty() => {
                    stats.exchanges += 1;
                    stats.approx_tokens += (follow.len() + r.len()) / 4;
                    stats.real_tokens += (u.input_tokens + u.output_tokens) as usize;
                    stats.tok_in += u64::from(u.input_tokens);
                    stats.tok_out += u64::from(u.output_tokens);
                    stats.cost_microusd += u.cost_microusd;
                    *usage = u; // latest context fill, mirroring the primary dial's repair call
                    tick_base += (r.chars().count() as u64) / 4;
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

//! Mid-relay tool calls. When a [`ToolRunner`] is attached, every agent's
//! task advertises the available tools; an agent calls one by ending its
//! reply with `@tool <server>:<tool> {"arg": …}`, or several reads on
//! consecutive last lines (`toolbatch`). The engine executes the calls and
//! re-dials the same agent with the results — up to [`MAX_TOOL_ROUNDS`]
//! calls per hop — before routing resumes. Every call and result is logged
//! as a hop, so tool use is visible in the pane.
use std::sync::Arc;

use super::adapter::{Adapter, HopStream, Usage};
use super::hop::{back, Hop, HopKind, RunStats};
use super::route::clip;
use super::toolbatch::{call_card, refusal, result_card, Ran, PER_REPLY};
use super::{Broker, Envelope};
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

/// Chars of tool output a result hop carries. The same budget the swarm's
/// result card uses, for the same reason: the card is folded to its outcome
/// line until clicked, so a long result costs scroll the reader asked for.
/// It was 400 — a display clip from before results were foldable, which cut
/// most real output off mid-word.
pub(crate) const RESULT_CLIP: usize = 4_000;

/// Most tool calls one agent may make within a single hop, each call of a
/// batch counted. Shared with the swarm so both engines stop at the same
/// number.
pub(crate) use crew_hive::tools::MAX_TOOL_ROUNDS;

/// The TOOLS prompt section for `tools` (empty when there are none).
pub(crate) fn hint_for(tools: &[McpTool]) -> String {
    if tools.is_empty() {
        return String::new();
    }
    let lines: Vec<String> = tools
        .iter()
        .map(|t| {
            // FIRST LINE, then clipped. `McpTool::description` is now the
            // server's whole description — paragraphs and all — and pasting
            // that into a one-line-per-tool list would break the list.
            let one = t.description.lines().next().unwrap_or("");
            format!("- {}:{} \u{2014} {}", t.server, t.name, clip(one, 100))
        })
        .collect();
    // The LOOK FIRST line is what turns a tool list into grounded answers.
    // Offered as merely "optional", a model asked what a crate in the repo
    // does answered from its priors (a web framework that does not exist)
    // with `sys:read_file` sitting right there.
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

/// The parser, shared with the swarm for the same reason the trait is: one
/// spelling of `@tool`, or an agent that works on one engine and not the other.
/// The split form, because the relay needs the text around the calls as well.
pub(crate) use crew_hive::tools::split_tool_calls;

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
    /// `stream` is the SAME `HopStream` the primary dial was given (built
    /// once at the engine call site) — reused rather than rebuilt per
    /// follow-up so the per-agent 150ms tick gate spans the whole hop,
    /// primary dial plus every follow-up. But that gate only ever emits on
    /// GROWTH, and each `call_with_usage_ticked` restarts its own chars/4
    /// estimate at 0 — so a follow-up dial can't just report its own running
    /// total, or a short follow-up would never climb past a long primary
    /// dial's last tick and would tick zero times for its whole duration.
    /// Each follow-up dial instead reports an OFFSET token estimate: its own
    /// total plus `tick_base`, the running sum of every prior dial's final
    /// chars/4 in this hop (primary dial included). That keeps every
    /// follow-up's reported value monotonically past wherever the hop's
    /// shared gate left off, so it survives the growth check instead of
    /// being swallowed by it. The TEXT side needs no such offset: fragments
    /// are appended to a buffer, never compared against a running total, so
    /// `stream.on_text` is reused as-is across every follow-up dial.
    #[allow(clippy::too_many_arguments)] // engine-loop plumbing, one call site
    pub(crate) fn run_tools(
        &self,
        agent: &dyn Adapter,
        base_prompt: &str,
        mut reply: String,
        stats: &mut RunStats,
        usage: &mut Usage,
        env: &Envelope,
        stream: &HopStream,
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
        // Kept in parts, so older results can be shortened in each prompt.
        let mut exchanges = crew_hive::tools::exchanges::Exchanges::default();
        // The reads this turn has made, so a repeat is not run again.
        let mut seen = crew_hive::tools::seen::Seen::default();
        let max_calls = self.tool_rounds;
        let mut used: u32 = 0;
        let before = loop {
            let Some((said, calls)) = split_tool_calls(&reply) else {
                return reply;
            };
            if used >= max_calls {
                break said;
            }
            // Each call counts, so a batch runs only as far as the budget
            // (and the per-reply cap) reaches; the rest are answered.
            let fit = calls.len().min(PER_REPLY).min((max_calls - used) as usize);
            for call in &calls[..fit] {
                sink(call_card(env, call));
            }
            let mut ran = super::toolbatch::run(runner, &mut seen, &exchanges, &calls[..fit]);
            let why = refusal(fit == (max_calls - used) as usize, max_calls);
            ran.resize_with(calls.len(), || Ran::not_run(&why));
            let mut round = Vec::with_capacity(calls.len());
            for (k, (call, r)) in calls.iter().zip(&ran).enumerate() {
                stats.approx_tokens += r.text.len() / 4;
                sink(result_card(env, call, r));
                // The message was written once, with the first call.
                let said = if k == 0 { said.as_str() } else { "" };
                let label = call.label();
                round.push(super::toolround::exchange(
                    said, &label, &call.args, &r.text,
                ));
            }
            exchanges.push_round(round);
            used += fit as u32;
            // The agent is TOLD what it has left. A budget it cannot see is
            // one it plans straight past, and then the turn ends mid-sequence
            // with a tool call nobody ran.
            let left = max_calls - used;
            let budget = if left == 0 {
                "This was your LAST tool call this turn: answer with what you \
                 have now."
                    .to_string()
            } else {
                format!("You may make {left} more tool call(s) this turn.")
            };
            let follow = format!(
                "{base_prompt}\n\nTOOL EXCHANGES THIS TURN:\n{}\n\n{budget} Continue the \
                 task using these results. You may call another tool, or answer and end \
                 with your routing line (`@next <agent>` or `@done`).",
                exchanges.render()
            );
            let label = calls[fit - 1].label();
            sink(Hop {
                from: label,
                to: env.to.clone(),
                hop: env.hop,
                kind: HopKind::Dialing,
                text: String::new(),
                usage: Default::default(),
            });
            let base = tick_base;
            let ticked = HopStream {
                // Tokens need the running offset (see this fn's doc): each
                // dial restarts its own chars/4 estimate at 0, and the gate
                // only emits on growth.
                on_tokens: {
                    let on = Arc::clone(&stream.on_tokens);
                    Arc::new(move |t| on(base + t))
                },
                // Text needs NO offset — fragments are appended, not
                // compared against a running total. Nor does reasoning.
                on_text: Arc::clone(&stream.on_text),
                on_thought: Arc::clone(&stream.on_thought),
                on_tool: Arc::clone(&stream.on_tool),
            };
            match agent.call_with_usage_ticked(&follow, self.timeout, &ticked) {
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
        };
        // Out of the loop means the last reply asked for MORE tools and the
        // budget is gone. It used to be returned as-is: the pane showed an
        // agent's unrun `@tool` line as its answer, with nothing anywhere
        // saying why it stopped halfway through what it was doing. Now the
        // note says why, and the answer is the text above the calls.
        sink(back(
            env,
            HopKind::Terminated,
            format!(
                "tool budget spent \u{2014} {max_calls} calls in one turn; \
                 the last request was not run"
            ),
        ));
        super::toolround::budget_answer(&before, max_calls)
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

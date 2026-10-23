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
use super::toolbatch::{refusal, Ran, PER_REPLY};
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
    /// call or the round cap trips, or the thread is stopped (`/stop`), which
    /// is looked at before each round's tools and before each follow-up
    /// dial. Returns the reply routing should parse.
    ///
    /// `usage` is the surrounding hop's usage (the primary dial's, or the
    /// repair dial's when it ran) — each follow-up dial overwrites it with
    /// its own real usage, the same "latest wins" rule the primary dial uses
    /// for its own repair call, so the hop's reply-stat and context-fill
    /// stay accurate through tool rounds. A fallback any dial of the hop
    /// made is kept ([`Usage::latest`]), so smith can still say so.
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
        // What every follow-up starts with; loses its recalled turns if one
        // outgrows the model's context (`toolfull`).
        let mut base = base_prompt.to_string();
        // Kept in parts, so older results can be shortened in each prompt.
        let mut exchanges = crew_hive::tools::exchanges::Exchanges::default();
        // The reads this turn has made, so a repeat is not run again.
        let mut seen = crew_hive::tools::seen::Seen::default();
        // The budget is ROUND TRIPS, each re-sending the whole prompt, not
        // calls: counted per call, a rename's six edits spent a solo turn's
        // eight before its last file (live, 2026-10-20), and the agent that
        // batched its edits paid for them as if it had not.
        let max_rounds = self.tool_rounds;
        let (mut used, mut made): (u32, u32) = (0, 0);
        // What the user typed while this turn ran (`steer`), kept for its rest.
        let mut steered: Vec<String> = Vec::new();
        let before = loop {
            let Some((said, calls)) = split_tool_calls(&reply) else {
                return reply;
            };
            // A stop pressed while the agent was dialed: what it asked for
            // is not run.
            if self.cancelled() {
                return super::toolround::stopped_answer(&said, made);
            }
            if used >= max_rounds {
                break said;
            }
            // A batch runs as far as the per-reply cap; the rest are answered.
            let fit = calls.len().min(PER_REPLY);
            for call in calls[..fit].iter().filter(|c| super::planout::carded(c)) {
                super::toolhive::call(stream, env, call);
            }
            let mut ran = super::toolbatch::run(runner, &mut seen, &exchanges, &calls[..fit]);
            let why = refusal();
            ran.resize_with(calls.len(), || Ran::not_run(&why));
            let mut round = Vec::with_capacity(calls.len());
            for (k, (call, r)) in calls.iter().zip(&ran).enumerate() {
                stats.approx_tokens += r.text.len() / 4;
                match (k < fit, super::planout::carded(call)) {
                    (false, _) => {}
                    (true, true) => super::toolhive::result(stream, env, call, r),
                    (true, false) => super::planout::send(stream, &env.to, call, r),
                }
                // The message was written once, with the first call.
                let said = if k == 0 { said.as_str() } else { "" };
                let label = call.label();
                round.push(super::toolround::exchange(
                    said, &label, &call.args, &r.text,
                ));
            }
            exchanges.push_round(round);
            (used, made) = (used + 1, made + fit as u32);
            // A stop pressed while the tools ran: no follow-up dial. Its
            // tools are not undone, and one still running was waited for:
            // `ToolRunner::call` takes no stop, and `sys:run` kills its
            // child only at its own deadline.
            if self.cancelled() {
                return super::toolround::stopped_answer(&said, made);
            }
            // Refused for its length, the follow-up is cut and dialed once
            // more (`toolfull`); the cut holds for the rest of the turn.
            let label = calls[fit - 1].label();
            let list = crew_hive::tools::todo::section(runner.checklist());
            steered.extend(super::steer::take());
            let added = super::steer::section(&steered);
            let (follow, dialed) = self.dial_fitted(
                agent,
                &mut base,
                &list,
                &mut exchanges,
                &added,
                max_rounds - used,
                &label,
                tick_base,
                env,
                stream,
                sink,
            );
            match dialed {
                Ok((r, u)) if !r.trim().is_empty() => {
                    stats.add((follow.len() + r.len()) / 4, &u);
                    usage.latest(u); // latest context fill, mirroring the primary dial's repair call
                    tick_base += (r.chars().count() as u64) / 4;
                    reply = r;
                }
                Ok(_) => {
                    sink(back(
                        env,
                        HopKind::Error,
                        "empty reply after tool call".into(),
                    ));
                    return super::toolround::failed_answer(&said, "an empty reply");
                }
                Err(e) => {
                    let answer = super::toolround::failed_answer(&said, &e);
                    sink(back(env, HopKind::Error, e));
                    return answer;
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
                "tool budget spent \u{2014} {max_rounds} rounds of tool calls in \
                 one turn; the last request was not run"
            ),
        ));
        super::toolround::budget_answer(&before, max_rounds)
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

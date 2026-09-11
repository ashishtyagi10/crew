//! The lead's one answer at the end of a multi-task run.
//!
//! A clean swarm used to end with nothing from agent smith: each specialist's
//! reply streamed as its own message, and when the plan ended in one sink
//! task (a merge) that reply WAS the answer. But the LLM planner often plans
//! two or three independent tasks and no merge, and then a single question
//! gets N separate replies and the user does the merging in their head. The
//! brain of the operation went quiet at exactly the step that needed it.
//!
//! So: after a clean run of two or more tasks whose graph does not already
//! end in one sink, ONE bounded call on the same plumbing as routing
//! (`intent::live_call` — cheap tier, 30 s, `None` under keyless/mock/
//! `CREW_INTENT=0`) writes the answer from the workers' outputs, and it lands
//! as agent smith's message after the per-task replies. Keyless and mock runs
//! make no call and emit no line, so their event stream is byte-identical to
//! before; a call that fails is one quiet line, never a failed run. A child
//! of `swarm`, so `broker` items are reached through `crate::broker::`.
use crew_hive::{RunOutcome, TaskGraph, TaskId, TaskResult};

use super::SWARM_LEAD;
use crate::broker::relay::msg;
use crate::protocol::PluginEvent;

/// A bounded one-shot the closing call may use.
pub(crate) type SynthFn = dyn Fn(&str) -> Result<String, String>;
/// The call, or `None` when none may run (keyless, mock, `CREW_INTENT=0`).
pub(crate) type Synth<'a> = Option<&'a SynthFn>;

/// Output ceiling for the answer: a few paragraphs, not a report — the
/// workers' replies are still in the pane above it for the long form.
const SYNTH_MAX_TOKENS: u32 = 768;
/// Chars of the user's request carried into a brief; mirrors `route::TASK_CAP`.
pub(super) const GOAL_CAP: usize = 4_000;
/// Chars of any ONE worker's output carried into the brief; mirrors crew-hive's
/// `DEP_CAP`, the budget a worker itself gets of each dependency's output.
const OUTPUT_CAP: usize = 4_000;
/// Chars of ALL workers' outputs together, shared evenly when the sum would
/// bust it; mirrors `DEPS_TOTAL_CAP`. Keeps the whole brief inside a
/// small-context model's window on a wide plan.
const OUTPUTS_TOTAL_CAP: usize = 12_000;
/// A failure reason is one line in the pane; a provider that rambles is cut.
const ERR_MAX: usize = 120;
/// The session log's name for the answer. The log skips the lead's own voice —
/// routing and plan lines are chrome — and the answer is the one thing it
/// says that is not, so it goes in under a name the filter lets through.
const ANSWER_SENDER: &str = "answer";

/// The live closing call, on routing's gates: `None` keyless, mock, or off.
pub(super) fn live() -> Option<Box<SynthFn>> {
    let call = crate::broker::intent::live_call(SYNTH_MAX_TOKENS)?;
    let boxed: Box<SynthFn> = Box::new(call);
    Some(boxed)
}

/// Whether the run needs the lead's answer: two or more tasks finished and
/// the graph does not end in exactly one sink. One sink is the planner's own
/// merge, and its reply already streamed as the answer — saying it again
/// would be the duplicate the old silence was protecting against.
pub(super) fn wants_answer(graph: &TaskGraph, done: &[TaskId]) -> bool {
    let sinks = done
        .iter()
        .filter(|t| !graph.tasks().iter().any(|s| s.deps.contains(t)))
        .count();
    done.len() >= 2 && sinks != 1
}

/// The lead's brief: the request, then every finished task's title and output
/// under the budgets above. Only the workers' outputs — the lead is combining
/// what the crew found, not finding more.
pub(super) fn prompt(goal: &str, parts: &[(String, String)]) -> String {
    format!(
        "You lead a crew of workers who each did one part of the user's request. Write \
         the answer to the request, drawn ONLY from the workers' outputs below: merge \
         them, resolve overlap, keep every concrete detail that answers the request and \
         drop what does not. Be concise. No preamble and no commentary about the workers \
         or the process \u{2014} the answer only.\n\nREQUEST:\n{}\n\nWORKER OUTPUTS:{}",
        clip_head(goal, GOAL_CAP),
        outputs(parts)
    )
}

/// The outputs half of a brief — one `## title` block per part, each held to
/// its share of the budgets above. Shared with the judge's and the revision's
/// briefs (`swarmverify`), so every reader of a run's outputs clips alike.
pub(super) fn outputs(parts: &[(String, String)]) -> String {
    let each = OUTPUT_CAP.min(OUTPUTS_TOTAL_CAP / parts.len().max(1));
    parts
        .iter()
        .map(|(title, output)| format!("\n\n## {title}\n{}", clip_head(output, each)))
        .collect()
}

/// Finished tasks as `(title, output)`, whole, in plan order (not completion
/// order, so the same plan briefs the same way twice). A task the graph no
/// longer knows (re-planned in) keeps its id as its title.
pub(super) fn parts<'a>(
    graph: &TaskGraph,
    results: impl Iterator<Item = &'a TaskResult>,
) -> Vec<(String, String)> {
    let title = |id: TaskId| {
        graph
            .get(id)
            .map_or_else(|| format!("task {}", id.0), |t| t.title.clone())
    };
    let mut parts: Vec<(TaskId, String, String)> = results
        .map(|r| (r.task, title(r.task), r.output.clone()))
        .collect();
    parts.sort_by_key(|p| p.0);
    parts.into_iter().map(|(_, t, o)| (t, o)).collect()
}

/// Say the answer, if the run needs one and a call may run. `results` are the
/// finished tasks' outputs, whole (the board is the archive; the budget is
/// applied here). Emits the lead thinking, the line, the lead idle — the shape
/// `intent::decision::announce` uses, so the pane shows smith working.
/// Returns the answer when one was written, so a judge can read it instead of
/// the outputs it merged.
pub(super) fn combine(
    goal: &str,
    graph: &TaskGraph,
    results: &[TaskResult],
    synth: Synth<'_>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<Option<String>> {
    let done: Vec<TaskId> = results.iter().map(|r| r.task).collect();
    let Some(call) = synth.filter(|_| wants_answer(graph, &done)) else {
        return Ok(None);
    };
    let parts = parts(graph, results.iter());
    emit(activity("thinking", "user"))?;
    let reply = call(&prompt(goal, &parts));
    let answer = match &reply {
        Ok(t) if !t.trim().is_empty() => Some(t.trim().to_owned()),
        _ => None,
    };
    if let Some(text) = &answer {
        crate::broker::sessionlog::append(ANSWER_SENDER, text);
    }
    let line = match (&answer, reply) {
        (Some(text), _) => text.clone(),
        (None, Ok(_)) => {
            "could not combine the workers' answers: the model returned nothing".into()
        }
        (None, Err(e)) => format!(
            "could not combine the workers' answers: {}",
            crate::broker::route::clip(&e, ERR_MAX)
        ),
    };
    emit(msg(SWARM_LEAD, line))?;
    emit(activity("idle", ""))?;
    Ok(answer)
}

/// The status line for a run that did not end clean — a cancellation or a
/// failure, which are not otherwise obvious. A clean run gets none: its
/// answer is the sinks' own replies, or [`combine`]'s line, and a "swarm
/// done" would be chrome.
pub(super) fn closing_line(outcome: &RunOutcome, cancelled: bool) -> Option<String> {
    if cancelled {
        Some(format!(
            "swarm cancelled (budget or /stop) — {} done, {} failed, {} cancelled",
            outcome.done.len(),
            outcome.failed.len(),
            outcome.cancelled.len()
        ))
    } else if !outcome.failed.is_empty() {
        Some(format!(
            "swarm finished with {} failed task(s)",
            outcome.failed.len()
        ))
    } else {
        None
    }
}

/// Agent smith's own activity — the pane's header pulse while the lead works.
pub(super) fn activity(state: &str, from: &str) -> PluginEvent {
    PluginEvent::Activity {
        agent: SWARM_LEAD.into(),
        state: state.into(),
        from: from.into(),
    }
}

/// The head of `s`, at most `max` chars, with a visible marker when cut.
/// Chars, never bytes, so multi-byte text is never split; under budget it
/// passes through byte-identical. `route::clip` is not used here because it
/// flattens whitespace, and a worker's output is often a list.
pub(super) fn clip_head(s: &str, max: usize) -> String {
    let total = s.chars().count();
    if total <= max {
        return s.to_owned();
    }
    let head: String = s.chars().take(max).collect();
    format!("{head}\u{2026} [clipped {} chars]", total - max)
}

#[cfg(test)]
#[path = "swarmanswer_tests.rs"]
mod tests;

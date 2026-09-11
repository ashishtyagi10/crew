//! Agent smith checks the swarm's work when the request said what "done"
//! looks like, and sends the crew back once when it is not.
//!
//! WHY: a swarm ended when its last task ended. "Make the tests pass" ran
//! three tasks, merged their replies, and closed — whether or not the tests
//! passed. The only judge in the system lived in `goal`, so the user had to
//! know to phrase the request as one. Now the router's `VERIFY: yes` line
//! (the model's call, see `intent::hints`) asks for a check: after a clean
//! run, ONE bounded judge call reads the request against the closing answer
//! (or the sink outputs when no answer was written) and rules `MET` or
//! `NOT MET: <reason>` through `constructs::parse_verdict`. Met is one quiet
//! line. Not met is one quiet line and one revision: the same planner,
//! factory and closing call, on a task that names the gap and carries what
//! was already done. The judge is the driver; [`REVISION_CAP`] is the
//! backstop that keeps a stubborn judge from becoming a loop.
//!
//! Keyless and mock runs get no judge (routing's gates), so their event
//! stream is byte-identical to before; a judge that errors is one quiet
//! line, never a failed run. A child of `swarm`, so `broker` items are
//! reached through `crate::broker::`.
use crew_hive::{TaskGraph, TaskId, TaskResult};

use super::swarmanswer::{self, SynthFn};
use super::SWARM_LEAD;
use crate::broker::constructs::parse_verdict;
use crate::broker::relay::msg;
use crate::protocol::PluginEvent;

/// Output ceiling for the verdict: one line, with room for a reason.
const JUDGE_MAX_TOKENS: u32 = 384;
/// How many times a `NOT MET` verdict may send the crew back within one
/// request. One: the revision is framed to close the gap the judge named,
/// and a second miss is the user's to read, not the swarm's to keep
/// spending on. The pass after the last revision is not judged at all.
pub(crate) const REVISION_CAP: u8 = 1;
/// A judge failure is one line in the pane; a provider that rambles is cut.
const ERR_MAX: usize = 120;
/// The session log's name for a verdict — a name the log's chrome filter
/// lets through, as `swarmanswer::ANSWER_SENDER` is for the answer.
const VERDICT_SENDER: &str = "verdict";

/// The judge for a run: the call, and how many revisions it may still ask
/// for. `Copy` so the run can hand the next pass its successor.
#[derive(Clone, Copy)]
pub(crate) struct Judge<'a> {
    call: &'a SynthFn,
    revisions: u8,
}

/// A run's verification: `None` — no `VERIFY: yes`, keyless, mock — and the
/// run ends exactly as it always did.
pub(crate) type Verify<'a> = Option<Judge<'a>>;

impl<'a> Judge<'a> {
    /// A fresh judge with the full revision allowance.
    pub(crate) fn new(call: &'a SynthFn) -> Self {
        Judge {
            call,
            revisions: REVISION_CAP,
        }
    }

    /// The judge for the pass after a revision: one fewer to spend, or none —
    /// the last allowed pass is not judged, so the cap is a hard ceiling.
    fn next(self) -> Verify<'a> {
        (self.revisions > 1).then(|| Judge {
            call: self.call,
            revisions: self.revisions - 1,
        })
    }
}

/// The live judge, on routing's gates, when the router asked for one.
pub(super) fn live(wanted: bool) -> Option<Box<SynthFn>> {
    if !wanted {
        return None;
    }
    let call = crate::broker::intent::live_call(JUDGE_MAX_TOKENS)?;
    let boxed: Box<SynthFn> = Box::new(call);
    Some(boxed)
}

/// The judge's brief: the request and the result, then the verdict grammar
/// `constructs::parse_verdict` reads.
pub(super) fn judge_prompt(goal: &str, result: &str) -> String {
    format!(
        "You are the judge. The user asked:\n{}\n\nThe crew's result:\n{result}\n\nIs the \
         request fully met \u{2014} every stated condition satisfied, not merely attempted? \
         Reply with exactly one line: `MET: <why>` or `NOT MET: <what is missing>`.",
        swarmanswer::clip_head(goal, swarmanswer::GOAL_CAP)
    )
}

/// The revision's task: the judge's gap first, what stands already, then the
/// goal itself — so the planner plans the difference, not the whole again.
pub(super) fn revise_task(goal: &str, reason: &str, outputs: &str) -> String {
    format!(
        "REVISE: the goal below was attempted; a judge found: {reason}.\n\nCompleted so \
         far:{outputs}\n\nDo only what closes the gap.\n\nGOAL:\n{}",
        swarmanswer::clip_head(goal, swarmanswer::GOAL_CAP)
    )
}

/// A task nothing depends on — its output is what the run hands the user.
fn is_sink(graph: &TaskGraph, id: TaskId) -> bool {
    !graph.tasks().iter().any(|s| s.deps.contains(&id))
}

/// Judge a clean run and say the verdict. `answer` is the lead's closing
/// answer when one was written; else the sink outputs stand for the result.
/// Returns the revision to run — its task and the judge for that pass — on
/// `NOT MET`; `None` on met, on a judge error (one quiet line), or when no
/// revision remains. Emits smith thinking, the line, smith idle.
pub(super) fn verdict<'a>(
    goal: &str,
    graph: &TaskGraph,
    results: &[TaskResult],
    answer: Option<&str>,
    judge: Judge<'a>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<Option<(String, Verify<'a>)>> {
    let sinks = swarmanswer::parts(graph, results.iter().filter(|r| is_sink(graph, r.task)));
    let outputs = swarmanswer::outputs(&sinks);
    let result = answer.map_or_else(|| outputs.trim().to_owned(), str::to_owned);
    emit(swarmanswer::activity("thinking", "user"))?;
    let (line, revise) = match (judge.call)(&judge_prompt(goal, &result)) {
        Ok(reply) => {
            let (met, why) = parse_verdict(&reply);
            let line = match (met, why.trim()) {
                (true, w) if w.is_empty() || w.eq_ignore_ascii_case("met") => "verified".into(),
                (true, w) => format!("verified \u{2014} {w}"),
                (false, w) => format!("not yet \u{2014} {w}"),
            };
            crate::broker::sessionlog::append(VERDICT_SENDER, &line);
            let revise = (!met).then(|| (revise_task(goal, why.trim(), &outputs), judge.next()));
            (line, revise)
        }
        Err(e) => (
            format!(
                "could not verify: {}",
                crate::broker::route::clip(&e, ERR_MAX)
            ),
            None,
        ),
    };
    emit(msg(SWARM_LEAD, line))?;
    emit(swarmanswer::activity("idle", ""))?;
    Ok(revise)
}

#[cfg(test)]
#[path = "swarmverify_tests.rs"]
mod tests;

//! The reply judge checks until the condition holds, and stops when nothing
//! moves.
//!
//! WHY: the judge (`replyverify`) sent a `NOT MET` back once and never read
//! the revision, so a revision that closed half the gap ended exactly like
//! one that closed all of it. Now every revision is judged too, up to
//! [`MAX_CHECKS`] calls. A loop the judge drives needs a reason to stop
//! other than `MET`, or one stubborn verdict spends the whole cap every
//! time: the same gap named twice means the revision did not move it, and a
//! revision that ran no tool and said the same again changed nothing the
//! judge could see, so judging it would only buy the last verdict again. One
//! that ran no tool but answered anew is judged: a gap can be in the words
//! ("the answer does not list the files"). Either stop, and the cap, is one
//! quiet line; `/stop` and a judge that errors end it as they always did.
use crate::broker::constructs::parse_verdict;
use crate::broker::relay::msg;
use crate::broker::route::clip;
use crate::broker::wording::count;
use crate::PluginEvent;

use super::{activity, prompt, revise, said, LEAD};

/// Judge calls one reply may spend: the first verdict, then one per
/// revision, so at most two revisions.
pub(crate) const MAX_CHECKS: usize = 3;
/// A judge failure is one line in the pane; a provider that rambles is cut.
const ERR_MAX: usize = 120;

/// What a turn left for the judge: its answer, and what its tools did
/// (`turnproof::lines`, empty when it ran none).
pub(crate) struct Turn {
    pub(crate) answer: String,
    pub(crate) proof: String,
}

/// What follows a verdict.
#[derive(Debug, PartialEq)]
pub(crate) enum Next {
    Met,
    /// Send the agent back on this gap.
    Revise(String),
    /// Another turn would not move anything: why, and the gap left open.
    Stalled(String),
}

/// What follows the judge's `(met, why)`, given the gap the check before it
/// named. The cap and the revision that ran nothing are the loop's to see:
/// one needs the count, the other is known before the judge is asked.
pub(crate) fn step(prev_gap: Option<&str>, verdict: (bool, &str)) -> Next {
    match verdict {
        (true, _) => Next::Met,
        (false, gap) if prev_gap.is_some_and(|p| words(p) == words(gap)) => {
            Next::Stalled(left("no progress", gap))
        }
        (false, gap) => Next::Revise(gap.trim().to_string()),
    }
}

/// `turn` on `task`, then judge and revise until the judge says `MET` or
/// [`step`] and the cap say stop. `turn` answers `None` when the task was
/// stopped or nobody answered — nothing to judge, so nothing is said.
pub(crate) fn run(
    task: &str,
    judge: &dyn Fn(&str) -> Result<String, String>,
    turn: &mut dyn FnMut(
        &str,
        &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<Option<Turn>>,
    stopped: &dyn Fn() -> bool,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let Some(mut last) = turn(task, emit)? else {
        return Ok(());
    };
    let mut prev: Option<String> = None;
    for check in 1..=MAX_CHECKS {
        if stopped() {
            break;
        }
        emit(activity("thinking", "user"))?;
        let reply = judge(&prompt(task, &last.proof, &last.answer));
        let (line, gap) = ruling(reply, prev.as_deref(), check == MAX_CHECKS);
        emit(msg(LEAD, line))?;
        emit(activity("idle", ""))?;
        let Some(gap) = gap else {
            break;
        };
        if stopped() {
            break;
        }
        let Some(next) = turn(&revise(task, &gap, &last.proof), emit)? else {
            break;
        };
        if next.proof.trim().is_empty() && words(&next.answer) == words(&last.answer) {
            return emit(msg(
                LEAD,
                stop_line(&left("the revision changed nothing", &gap)),
            ));
        }
        (last, prev) = (next, Some(gap));
    }
    Ok(())
}

/// One check, said: the pane's line, and the gap to send back when there is
/// one. A judge that errored sends nothing back — its silence is not a gap.
fn ruling(
    reply: Result<String, String>,
    prev: Option<&str>,
    last: bool,
) -> (String, Option<String>) {
    let (met, why) = match reply {
        Ok(r) => parse_verdict(&r),
        Err(e) => return (format!("could not verify: {}", clip(&e, ERR_MAX)), None),
    };
    match step(prev, (met, &why)) {
        Next::Met => (said("verified", &why), None),
        Next::Revise(gap) if last => {
            let used = format!("{} used", count(MAX_CHECKS, "check"));
            (stop_line(&left(&used, &gap)), None)
        }
        Next::Revise(gap) => (said("not yet", &gap), Some(gap)),
        Next::Stalled(why) => (stop_line(&why), None),
    }
}

/// `why: gap`, or bare `why` when the judge named no gap.
fn left(why: &str, gap: &str) -> String {
    match gap.trim() {
        "" => why.to_string(),
        g => format!("{why}: {g}"),
    }
}

fn stop_line(why: &str) -> String {
    format!("stopped checking \u{2014} {why}")
}

/// A gap as its words: lowercase, punctuation gone, so a verdict that only
/// re-cases or re-punctuates the last one still reads as the same gap.
fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

#[cfg(test)]
#[path = "replyloop_step_tests.rs"]
mod step_tests;
#[cfg(test)]
#[path = "replyloop_tests.rs"]
mod tests;

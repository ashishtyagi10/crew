//! What a task that failed hands back: the error, in words.
//!
//! A provider error used to end the task with an EMPTY output. The pane heard
//! why — the agent's `Failed` event carried it — but the scheduler passes the
//! output on as the failure, so the planner re-planning the rest read
//! `FAILED task "X": ` and nothing after the colon. A timeout, a rejected key
//! and a model the host does not serve all looked alike, and the usual answer
//! was the same task again. Now the output IS the reason, the same words the
//! pane shows, and the attempt says whether the failure passes, which is what
//! earns the task a second run (`sched::again`).
use super::{Ended, TurnContext};
use crate::bus::HiveEvent;
use crate::provider::ProviderError;

/// Longest reason a failed task hands back. A provider's sentence and its
/// hint fit in well under this; what does not is a body nobody boiled down,
/// and the planner needs the diagnosis, not the page.
pub(crate) const REASON_CAP: usize = 300;

/// Publish `err` as the agent's failure and hand it back as how the turn
/// ended — one string for both, so the pane and the planner hear the same.
pub(super) fn failed(ctx: &TurnContext, err: &ProviderError) -> Ended {
    let why = reason(err);
    ctx.publish(HiveEvent::Failed {
        agent: ctx.agent.clone(),
        error: why.clone(),
    });
    Ended::Failed {
        why,
        transient: err.is_transient(),
    }
}

/// `err` said in at most [`REASON_CAP`] characters. A longer one keeps its
/// whole lines when it has more than one to keep, so what reaches the
/// planner is never a sentence cut in half; a single overlong line is cut
/// and says so.
pub(crate) fn reason(err: &dyn std::fmt::Display) -> String {
    let said = err.to_string();
    let said = said.trim();
    if said.chars().count() <= REASON_CAP {
        return said.to_string();
    }
    let head: String = said.chars().take(REASON_CAP).collect();
    match head.rfind('\n') {
        Some(end) if !head[..end].trim().is_empty() => head[..end].trim_end().to_string(),
        _ => format!("{}\u{2026}", head.trim_end()),
    }
}

#[cfg(test)]
#[path = "failure_tests.rs"]
mod tests;

//! What a hop's turn comes back to `Broker::run` as: its answer with the
//! routing line last, or the error card a dial that failed would have sunk.
use crew_hive::harness::Ended;

use super::pane::Tally;
use crate::broker::adapter::Usage;
use crate::broker::hop::{back, Hop, HopKind};
use crate::broker::{cancelscope, route, toolround, Envelope};

/// What a hop whose turn `ended` having run up `tally` replies with: the
/// answer and usage for routing, or `None` when it ended on an error card
/// (sunk here), as a dial that failed. A turn that made a call before it
/// went quiet, failed or was stopped still answers, from what it wrote.
pub(super) fn of(
    ended: Ended,
    mut tally: Tally,
    env: &Envelope,
    sink: &mut dyn FnMut(Hop),
) -> Option<(String, Usage)> {
    let (made, wrote) = (tally.calls, std::mem::take(&mut tally.wrote));
    let usage = std::mem::take(&mut tally.usage);
    let error = |why: String| back(env, HopKind::Error, why);
    match ended {
        Ended::Answered(text) if !text.trim().is_empty() => Some((routed_last(&text), usage)),
        Ended::Answered(_) if made == 0 => {
            sink(error("empty reply".into()));
            None
        }
        Ended::Answered(_) => {
            sink(error("empty reply after tool call".into()));
            Some((toolround::failed_answer(&wrote, "an empty reply"), usage))
        }
        Ended::Failed { why, .. } => {
            let why = tally.said.take().unwrap_or(why);
            if made == 0 {
                sink(error(why));
                return None;
            }
            let answer = toolround::failed_answer(&wrote, &why);
            sink(error(why));
            Some((answer, usage))
        }
        Ended::Stopped if made == 0 => {
            sink(error(cancelscope::STOPPED.into()));
            None
        }
        Ended::Stopped => Some((toolround::stopped_answer(&wrote, made), usage)),
    }
}

/// `answer` with its routing line last again. The harness writes its own
/// notes under a reply — the checklist's unfinished steps, the spent budget,
/// a call that never ran (`crew_hive::tools::is_note`) —
/// and the reply's own last line was its `@done` or `@next`, the only line
/// routing reads (`route::parse_routing`): under a note, the `@done` stayed
/// in the answer as text and an `@next` hand-off was lost. The notes stay,
/// above it.
pub(super) fn routed_last(answer: &str) -> String {
    let lines: Vec<&str> = answer.trim().lines().collect();
    let note = |l: &str| l.trim().is_empty() || crew_hive::tools::is_note(l);
    let Some(at) = lines.iter().rposition(|l| !note(l)) else {
        return answer.trim().to_string();
    };
    let below = &lines[at + 1..];
    if below.is_empty() || !route::has_directive(lines[at]) {
        return answer.trim().to_string();
    }
    let above = lines[..at].join("\n");
    let notes = below.join("\n");
    format!(
        "{}\n\n{}\n{}",
        above.trim_end(),
        notes.trim(),
        lines[at].trim()
    )
    .trim()
    .to_string()
}

#[cfg(test)]
#[path = "hopreply_tests.rs"]
mod tests;

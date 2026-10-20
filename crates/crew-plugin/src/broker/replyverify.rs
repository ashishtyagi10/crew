//! A routed reply is checked against its request when the request says what
//! "done" is — the swarm's judge (`swarmverify`), for one agent's turn.
//!
//! WHY: a reply ended when its agent stopped. Live (2026-10-20, qwen3.8-max,
//! "rename relative_luminance … and make sure cargo test -p crew-theme
//! passes"), a turn renamed two of three files, never ran the tests, and
//! answered that the rename was complete. The router already says when a
//! request states a checkable condition (`VERIFY: yes`); a reply now gets the
//! same ONE bounded judge call a swarm does. The judge reads the request, the
//! answer and what the turn's tools did (`turnproof`), and a claim no tool
//! result shows is not enough. `MET` is one quiet line. `NOT MET` is one
//! quiet line and one more turn of the same agent on the gap the judge named;
//! that turn is not judged, so one stubborn verdict cannot become a loop.
//!
//! Keyless and mock runs get no judge (routing's gates), and a judge that
//! errors is one quiet line, never a failed turn.
use std::sync::Arc;

use super::constructs::parse_verdict;
use super::relay::msg;
use super::session::Session;
use crate::PluginEvent;

/// Output ceiling for the verdict: one line, with room for a reason.
const JUDGE_MAX_TOKENS: u32 = 384;
/// Chars of the answer and the request the judge reads.
const ANSWER_CAP: usize = 2_500;
const LEAD: &str = "agent smith";

/// Answer `task` as one agent's reply and, when `verify`, judge it once.
pub(crate) fn reply(
    task: &str,
    pick: Option<&str>,
    verify: bool,
    session: &Session,
    tick_emit: &Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let judge = verify
        .then(|| super::intent::live_call_at(JUDGE_MAX_TOKENS, crew_hive::ModelTier::Standard))
        .flatten();
    super::turnproof::start();
    let answer = super::stdio::relay_answer(task, pick, session, tick_emit, emit)?;
    let (Some(judge), Some(answer)) = (judge, answer) else {
        return Ok(());
    };
    emit(activity("thinking", "user"))?;
    let verdict = judge(&prompt(task, &super::turnproof::lines(), &answer));
    let gap = match verdict {
        Ok((reply, _)) => match parse_verdict(&reply) {
            (true, why) => {
                emit(msg(LEAD, said("verified", &why)))?;
                None
            }
            (false, why) => {
                emit(msg(LEAD, said("not yet", &why)))?;
                Some(why)
            }
        },
        Err(e) => {
            let e = super::route::clip(&e, 120);
            emit(msg(LEAD, format!("could not verify: {e}")))?;
            None
        }
    };
    emit(activity("idle", ""))?;
    match gap {
        Some(why) if !session.cancelled() => {
            super::turnproof::start();
            super::stdio::relay_answer(&revise(task, &why), pick, session, tick_emit, emit)
                .map(|_| ())
        }
        _ => Ok(()),
    }
}

/// The judge's brief: the request, what the tools did, the answer, and the
/// verdict grammar `parse_verdict` reads.
pub(crate) fn prompt(task: &str, proof: &str, answer: &str) -> String {
    let proof = match proof.trim() {
        "" => "(no tool calls)",
        p => p,
    };
    format!(
        "You are the judge. The user asked:\n{}\n\nWhat the agent's tools did this \
         turn, in order:\n{proof}\n\nThe agent's answer:\n{}\n\nIs the request fully \
         met \u{2014} every stated condition satisfied AND shown by a tool result \
         above, not merely claimed in the answer? Reply with exactly one line: \
         `MET: <why>` or `NOT MET: <what is missing>`.",
        super::route::clip(task, ANSWER_CAP),
        super::route::clip(answer, ANSWER_CAP)
    )
}

/// The second turn's task: the gap first, then the request it serves.
pub(crate) fn revise(task: &str, why: &str) -> String {
    format!(
        "A check of your last turn found: {why}\nDo only what closes that gap, and run \
         what proves it.\n\nTHE REQUEST:\n{task}"
    )
}

/// The verdict line: `verified — the tests ran green`, or bare when the judge
/// gave no reason.
fn said(word: &str, why: &str) -> String {
    match why.trim() {
        w if w.is_empty() || w.eq_ignore_ascii_case("met") => word.into(),
        w => format!("{word} \u{2014} {w}"),
    }
}

fn activity(state: &str, from: &str) -> PluginEvent {
    PluginEvent::Activity {
        agent: LEAD.into(),
        state: state.into(),
        from: from.into(),
    }
}

#[cfg(test)]
#[path = "replyverify_tests.rs"]
mod tests;

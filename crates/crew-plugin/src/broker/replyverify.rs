//! A routed reply is checked against its request when the request says what
//! "done" is — the swarm's judge (`swarmverify`), for one agent's turn.
//!
//! WHY: a reply ended when its agent stopped. Live (2026-10-20, qwen3.8-max,
//! "rename relative_luminance … and make sure cargo test -p crew-theme
//! passes"), a turn renamed two of three files, never ran the tests, and
//! answered that the rename was complete. The router already says when a
//! request states a checkable condition (`VERIFY: yes`); a reply now gets a
//! bounded judge the way a swarm does. The judge reads the request, the
//! answer and what the turn's tools did (`turnproof`), and a claim no tool
//! result shows is not enough. `MET` is one quiet line. `NOT MET` is one
//! quiet line and one more turn of the same agent on the gap the judge named,
//! and that revision is judged as the first turn was (`replyloop`).
//!
//! Keyless and mock runs get no judge (routing's gates), and a judge that
//! errors is one quiet line, never a failed turn.
use std::sync::Arc;

use super::session::Session;
use crate::PluginEvent;
use replyloop::Turn;

/// Output ceiling for the verdict: one line, with room for a reason.
const JUDGE_MAX_TOKENS: u32 = 384;
/// Chars of the answer and the request the judge reads.
const ANSWER_CAP: usize = 2_500;
const LEAD: &str = "agent smith";
/// The most of the last turn's tool lines a revision is given.
const PROOF_TAIL: usize = 12;

/// Answer `task` as one agent's reply and, when `verify`, judge it until it
/// holds (`replyloop::run`).
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
    // Each turn's proof is its own: the log starts empty before it runs.
    let mut turn = |t: &str, emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>| {
        super::turnproof::start();
        let answer = super::stdio::relay_answer(t, pick, session, tick_emit, emit)?;
        let proof = super::turnproof::lines();
        anyhow::Ok(answer.map(|answer| Turn { answer, proof }))
    };
    let Some(judge) = judge else {
        return turn(task, emit).map(|_| ());
    };
    let judge = |p: &str| judge(p).map(|(reply, _)| reply);
    replyloop::run(task, &judge, &mut turn, &|| session.cancelled(), emit)
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

/// A revision's task: the gap first, then what the last turn's tools did,
/// then the request it serves. Without the tools' record a revision began
/// from the gap alone: 7 of 19 on 2026-10-23 opened by re-running the test
/// the last turn had just run, and they took 3.4 rounds to land an edit.
pub(crate) fn revise(task: &str, why: &str, proof: &str) -> String {
    let lines: Vec<&str> = proof.lines().filter(|l| !l.trim().is_empty()).collect();
    let tail = &lines[lines.len().saturating_sub(PROOF_TAIL)..];
    let did = match tail.len() {
        0 => String::new(),
        n => format!(
            "\n\nWHAT YOUR LAST TURN'S TOOLS DID (the last {n} of {}):\n{}",
            lines.len(),
            tail.join("\n")
        ),
    };
    format!(
        "A check of your last turn found: {why}\nDo only what closes that gap, and run \
         what proves it.{did}\n\nTHE REQUEST:\n{task}"
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

#[path = "replyloop.rs"]
mod replyloop;

#[cfg(test)]
#[path = "replyverify_tests.rs"]
mod tests;

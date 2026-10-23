//! One relay tool round: running the call, how the agent's next prompt
//! records it, and what a turn answers with when its tool budget runs out.
//!
//! Kept beside `toolcall.rs` rather than in it: the loop there is about
//! dialing and streaming, and these are the two places where a reply's text
//! is cut around its `@tool` call, so they share one reading of where the call
//! sits (`crew_hive::tools::split_tool_call`) with the swarm.
use super::toolcall::ToolRunner;
use super::toolclip::{clip_result, AGENT_CLIP};
use crew_hive::tools::exchanges::{Exchange, Exchanges};
use crew_hive::tools::{seen::Seen, ToolCall};

/// Run `call` as entry `entry` (from 1) of this turn's log: `(ok, text,
/// repeat)`.
///
/// A read this turn already made, with nothing written since, is not run
/// while `log` still shows its result whole: its text is a pointer to the
/// earlier result, and `repeat` names that round so the card can say so
/// (`Seen`, `Exchanges::repeat`).
pub(super) fn run_once(
    runner: &dyn ToolRunner,
    seen: &mut Seen,
    log: &Exchanges,
    call: &ToolCall,
    entry: u32,
) -> (bool, String) {
    if let Some(first) = log.repeat(seen, call) {
        return (true, Seen::pointer(first));
    }
    if let Some((round, why)) = seen.missed(call) {
        return (false, Seen::again(round, why));
    }
    let (ok, text) = settle(
        runner,
        call,
        runner.call(&call.server, &call.tool, &call.args),
    );
    seen.ran(runner, call, entry, ok);
    if !ok {
        seen.miss(call, entry, &text);
    }
    (ok, text)
}

/// What a call that ran came back as: `(ok, text)`.
///
/// An `Ok` can still be a failure (a build that exited 101), and only the
/// tool surface can say so (`Tools::failed`).
pub(super) fn settle(
    runner: &dyn ToolRunner,
    call: &ToolCall,
    called: Result<String, String>,
) -> (bool, String) {
    let ok = called
        .as_ref()
        .is_ok_and(|t| !runner.failed(&call.server, &call.tool, t));
    let text = match called {
        Ok(t) if t.is_empty() => "(empty result)".to_string(),
        Ok(t) => t,
        Err(e) => format!("ERROR: {e}"),
    };
    (ok, text)
}

/// One entry of the TOOL EXCHANGES log: what the agent wrote with the call
/// (`said`, the reply with the call cut out), then the call and its result.
///
/// The message was not there before, and the follow-up prompt read as a bare
/// list of calls. An agent that had written "the bug is in route.rs; reading
/// clip() next" came back to a result with no trace of why it asked for it,
/// and often asked for a file it had already read.
pub(super) fn exchange(said: &str, label: &str, args: &str, result: &str) -> Exchange {
    Exchange::new(said, label, args, clip_result(result, AGENT_CLIP))
}

/// The last line of a follow-up prompt: the rounds of calls `left` this turn,
/// and how to finish.
///
/// The agent is TOLD what it has left. A budget it cannot see is one it plans
/// straight past, and then the turn ends mid-sequence with a tool call nobody
/// ran. How to finish is pointed at, not restated: the line used to offer
/// `@next <agent>` to an agent alone on its turn, whose frame had just said
/// to end with `@done`, and "You may call another tool" followed "This was
/// your LAST tool call". The frame's HOW TO REPLY, alone or with peers, is
/// the one statement of it, so the follow-up cannot disagree with it.
pub(super) fn next_step(left: u32) -> String {
    match left {
        0 => "That was your LAST round of tool calls this turn: answer now with \
              what you have, and end the answer as HOW TO REPLY says."
            .to_string(),
        n => format!(
            "You may make {n} more round(s) of tool calls this turn, up to {} \
             calls in one reply, or answer now and end the answer as HOW TO \
             REPLY says.",
            super::toolbatch::PER_REPLY
        ),
    }
}

/// The answer of a turn whose last reply asked for a tool after the budget
/// was spent: `before`, the text the agent wrote above that call, without
/// the call.
///
/// It used to be the reply as written, so the pane showed raw `@tool` syntax,
/// a fence and pretty-printed JSON included, as the answer. When the call was
/// all the agent wrote there is nothing to answer with, and an empty answer
/// reads as a crash; one plain line says what happened instead.
///
/// Calls still in `before` (a group the reply wrote above prose and its last
/// calls) were refused by the budget too, and the note says so: the generic
/// one [`never_ran`] adds told the reader, and the reply judge, that the
/// turn "ended before any call in it ran", which sent the judge after a turn
/// that had run eight rounds (live, 2026-10-23).
pub(super) fn budget_answer(before: &str, rounds: u32) -> String {
    let stray = crew_hive::tools::stray_calls(before);
    match (before.trim(), stray.first()) {
        ("", _) => format!("stopped before answering: the tool budget ({rounds} rounds) ran out"),
        (kept, Some(first)) => format!(
            "{kept}\n\nnot run \u{2014} the tool budget ({rounds} rounds) was spent before \
             `{first}`{} could run",
            more(stray.len())
        ),
        (kept, None) => kept.to_string(),
    }
}

/// ` and 2 more`, after the first of `n` calls named.
fn more(n: usize) -> String {
    match n {
        0 | 1 => String::new(),
        n => format!(" and {} more", n - 1),
    }
}

/// The answer of a turn whose next model call failed after a tool round:
/// `before`, the text above the calls just run, and a line saying why it
/// stopped. It used to return the whole reply, so its raw `@tool` line
/// became the answer the user read.
pub(super) fn failed_answer(before: &str, why: &str) -> String {
    let note = format!("stopped \u{2014} the model's next reply failed: {why}");
    match before.trim() {
        "" => note,
        kept => format!("{kept}\n\n{note}"),
    }
}

/// The answer of a turn stopped between tool rounds (`/stop`): `before`, the
/// text the agent wrote above the calls it asked for last, and a line saying
/// how far the turn got. The stop used to wait out every round left, each a
/// whole model call; the note is what tells the reader the answer is partial.
pub(super) fn stopped_answer(before: &str, calls: u32) -> String {
    let s = if calls == 1 { "" } else { "s" };
    let note = format!("stopped \u{2014} {calls} tool call{s} made");
    match before.trim() {
        "" => note,
        kept => format!("{kept}\n\n{note}"),
    }
}

/// A turn's final answer as the user reads it: with one line under it naming
/// what the task's checklist left undone (`crew_hive::tools::todo`), so a
/// step the agent skipped is not hidden behind an answer that says done. The
/// answer as written when the task kept no list or finished it.
pub(super) fn finished(runner: Option<&dyn ToolRunner>, answer: String) -> String {
    let answer = crew_hive::tools::todo::finished(answer, runner.and_then(|r| r.checklist()));
    never_ran(answer)
}

/// `answer` with one line under it when it still holds a call: a finished
/// answer has run its trailing calls, so one left in it never ran, and the
/// work it says comes next was not done. The call stays as written.
pub(super) fn never_ran(answer: String) -> String {
    let stray = crew_hive::tools::stray_calls(&answer);
    let said = answer.trim_end().rsplit('\n').next();
    let (Some(first), false) = (
        stray.first(),
        said.is_some_and(|l| l.starts_with("not run")),
    ) else {
        return answer;
    };
    format!(
        "{}\n\nnot run \u{2014} the answer asks for `{first}`{}, but the turn \
         ended before any call in it ran",
        answer.trim_end(),
        more(stray.len())
    )
}

#[cfg(test)]
#[path = "toolround_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "samemiss_tests.rs"]
mod samemiss_tests;

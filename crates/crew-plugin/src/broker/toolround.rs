//! How a relay turn's answer ends, whatever ran it: the line under an answer
//! a failure or a stop cut short, the checklist's last word, and the note
//! under an answer that still holds a call nobody ran. The tool rounds
//! themselves are the harness's (`hopturn`).
use super::toolcall::ToolRunner;

/// The answer of a turn whose next model call failed after a tool round:
/// `before`, the text above the calls just run (empty when the harness kept
/// none), and a line saying why it stopped. It used to return the whole
/// reply, so its raw `@tool` line became the answer the user read.
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
/// work it says comes next was not done. The call stays as written. An
/// answer whose rounds ran out (the harness's budget note) says the budget
/// stopped it: the generic line told the reader, and the reply judge, that
/// the turn "ended before any call in it ran", which sent the judge after a
/// turn that had run every round it had (live, 2026-10-23). One that already
/// says what did not run says it once.
pub(super) fn never_ran(answer: String) -> String {
    let stray = crew_hive::tools::stray_calls(&answer);
    let said = answer.trim_end().rsplit('\n').next().unwrap_or("");
    let Some(first) = stray.first().filter(|_| !said.starts_with("not run")) else {
        return answer;
    };
    let why = match said.starts_with("[tool budget spent") {
        true => "the tool budget was spent before it could run",
        false => "the turn ended before any call in it ran",
    };
    format!(
        "{}\n\nnot run \u{2014} the answer asks for `{first}`{}, but {why}",
        answer.trim_end(),
        more(stray.len())
    )
}

/// ` and 2 more`, after the first of `n` calls named.
fn more(n: usize) -> String {
    match n {
        0 | 1 => String::new(),
        n => format!(" and {} more", n - 1),
    }
}

/// What a second model makes of a failed check, for the follow-up.
#[path = "advise.rs"]
pub(super) mod advise;

#[cfg(test)]
#[path = "toolround_tests.rs"]
mod tests;

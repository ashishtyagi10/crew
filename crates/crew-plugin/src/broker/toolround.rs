//! How a relay turn's answer ends, whatever ran it: the line under an answer
//! a failure or a stop cut short, the checklist's last word, and the note
//! under an answer that still holds a call nobody ran — both said by the
//! harness for a turn it ran, and here again, once, where the thread ends.
//! The tool rounds themselves are the harness's (`hopturn`).
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

/// `answer` with one line under it when it still holds a call that never
/// ran (`crew_hive::tools::never_ran`, where the harness finishes its own).
pub(super) fn never_ran(answer: String) -> String {
    crew_hive::tools::never_ran(answer)
}

#[cfg(test)]
#[path = "toolround_tests.rs"]
mod tests;

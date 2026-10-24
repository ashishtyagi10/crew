//! The line under an answer that still holds a call nobody ran.
//!
//! A finished answer has run its trailing calls, so one left in it — written
//! mid-answer, fenced as an example of what it meant to do next (live probe,
//! 2026-10-16) — never ran, and the work it says comes next was not done.
//! Both engines say so, where every answer is finished (`harness::finish`,
//! and the relay's thread end for a hop that ran no turn).

use super::stray_calls;

/// `answer` with one line under it when it still holds a call; the call stays
/// as written. An answer whose rounds ran out (the harness's budget note
/// among its last lines) says the budget stopped it: the generic line told
/// the reader, and the reply judge, that the turn "ended before any call in
/// it ran", which sent the judge after a turn that had run every round it had
/// (live, 2026-10-23). One that already says what did not run says it once.
pub fn never_ran(answer: String) -> String {
    let stray = stray_calls(&answer);
    let Some(first) = stray.first() else {
        return answer;
    };
    // The notes an answer ends on, newest last: the checklist's, the budget's,
    // this one's own.
    let notes: Vec<&str> = answer
        .trim_end()
        .lines()
        .rev()
        .take_while(|l| l.trim().is_empty() || is_note(l))
        .collect();
    if notes.iter().any(|l| l.starts_with("not run")) {
        return answer;
    }
    let why = match notes.iter().any(|l| l.starts_with(BUDGET)) {
        true => "the tool budget was spent before it could run",
        false => "the turn ended before any call in it ran",
    };
    let more = match stray.len() {
        0 | 1 => String::new(),
        n => format!(" and {} more", n - 1),
    };
    format!(
        "{}\n\nnot run \u{2014} the answer asks for `{first}`{more}, but {why}",
        answer.trim_end()
    )
}

/// How the harness's budget note opens (`toolloop::with_budget_note`).
const BUDGET: &str = "[tool budget spent";

/// A line one of the engines writes under an answer, never the agent: the
/// checklist's unfinished steps (`todo::finished`), the spent budget, a call
/// that never ran. The relay keeps its routing line below them (crew-plugin
/// `hopturn::routed_last`).
pub fn is_note(line: &str) -> bool {
    let l = line.trim();
    l.starts_with("checklist: ") || l.starts_with(BUDGET) || l.starts_with("not run \u{2014} ")
}

#[cfg(test)]
#[path = "neverran_tests.rs"]
mod tests;

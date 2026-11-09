//! The calls a finished answer still holds, lifted out and named once.
//!
//! A finished answer has run its trailing calls, so one left in it — written
//! mid-answer, fenced as an example of what it meant to do next (live probe,
//! 2026-10-16) — never ran, and the work it says comes next was not done.
//! Both engines say so, where every answer is finished (`harness::finish`,
//! and the relay's thread end for a hop that ran no turn).
//!
//! The call used to stay in the answer as written with a `not run — …`
//! sentence under it, and the pane drew both as the agent's own markdown: a
//! raw `@tool sys:read_file {…}` line the user could not act on, then the
//! engine's words in the agent's voice. The call now comes out whole (its
//! JSON, and the fence it leaves empty) and the note is smith's `!` line,
//! which the relay says as its own row under the reply (`Usage::not_run`).

use super::around::{is_fence, opens_fence};
use super::parse::calls_in;
use super::ToolCall;

/// How smith's line opens; how an answer is known to have been lifted.
const NOT_RUN: &str = "! not run: ";

/// How the harness's budget note opens (`toolloop::with_budget_note`).
const BUDGET: &str = "[tool budget spent";

/// A call's argument longer than this is clipped: the line names the call,
/// it does not repeat it.
const ARG_MAX: usize = 40;

/// `answer` without the calls it never ran, and smith's line naming them —
/// `! not run: sys:read_file README.md — the turn ended first` — or `None`
/// when it holds none. An answer an engine already lifted gives its line
/// back (the harness lifts, then the relay's thread end lifts again). One
/// whose rounds ran out (the harness's budget note among its last lines)
/// says the budget stopped them: "the turn ended" sent the reply judge after
/// a turn that had run every round it had (live, 2026-10-23).
pub fn not_run(answer: &str) -> (String, Option<String>) {
    let (body, said) = split_note(answer);
    let kept: Vec<&str> = body.lines().collect();
    let calls = calls_in(&kept);
    if calls.is_empty() {
        return (body, said);
    }
    let lines = &kept[kept.len() - tail_notes(&kept)..];
    let budget = lines.iter().any(|l| l.starts_with(BUDGET));
    let why = if budget {
        "the tool budget ran out first"
    } else {
        "the turn ended first"
    };
    let named: Vec<String> = calls.iter().map(|(_, _, c)| named(c)).collect();
    let mut gone = vec![false; kept.len()];
    for (at, used, _) in &calls {
        gone[*at..=at + used].iter_mut().for_each(|g| *g = true);
    }
    let left: Vec<&str> = (kept.iter().zip(&gone))
        .filter_map(|(l, g)| (!g).then_some(*l))
        .collect();
    let text = without_empty_fences(&left).trim_end().to_string();
    (
        text,
        Some(format!(
            "{NOT_RUN}{} \u{2014} {why}",
            named.join(" \u{b7} ")
        )),
    )
}

/// `answer` without the `! not run:` line an engine already wrote under it,
/// and that line — for a reader that only says it (the swarm's pane
/// messages), which must not lift a call from a keyless worker's answer,
/// where a `@tool` line is just text. The answer as it was when it has none.
pub fn split_note(answer: &str) -> (String, Option<String>) {
    let lines: Vec<&str> = answer.trim_end().lines().collect();
    let tail = lines.len() - tail_notes(&lines);
    let Some(said) = lines[tail..].iter().find(|l| l.starts_with(NOT_RUN)) else {
        return (answer.to_string(), None);
    };
    let kept: Vec<&str> = (lines.iter().enumerate())
        .filter(|&(i, l)| i < tail || !l.starts_with(NOT_RUN))
        .map(|(_, l)| *l)
        .collect();
    (
        kept.join("\n").trim_end().to_string(),
        Some(said.to_string()),
    )
}

/// `answer` with the calls it never ran lifted out and smith's line under
/// it, for an engine with no row of smith's to say it on (the swarm's
/// output, the reply judge). Said once, however often it is asked.
pub fn never_ran(answer: String) -> String {
    match not_run(&answer) {
        (text, None) => text,
        (text, Some(note)) if text.is_empty() => note,
        (text, Some(note)) => format!("{text}\n\n{note}"),
    }
}

/// How many of `lines`' last lines are the engines' notes or blank.
fn tail_notes(lines: &[&str]) -> usize {
    let n = lines.iter().rev();
    n.take_while(|l| l.trim().is_empty() || is_note(l)).count()
}

/// `sys:read_file README.md`: the call, and the one argument that says what
/// it was for — a path, a command, a query — clipped.
fn named(c: &ToolCall) -> String {
    let v: Option<serde_json::Value> = serde_json::from_str(&c.args).ok();
    let arg = match &v {
        Some(v) => purpose(v).unwrap_or_default(),
        None => c.args.as_str(),
    };
    let arg = arg.lines().next().unwrap_or_default().trim();
    let arg = match arg.char_indices().nth(ARG_MAX) {
        Some((cut, _)) => format!("{}\u{2026}", arg[..cut].trim_end()),
        None => arg.to_string(),
    };
    format!("{} {arg}", c.label()).trim_end().to_string()
}

/// The argument that says what a call was for: a named one first, else the
/// first string (serde_json orders keys, so "first" is not the model's).
fn purpose(v: &serde_json::Value) -> Option<&str> {
    let o = v.as_object()?;
    let keys = ["path", "cmd", "command", "args", "query", "pattern", "url"];
    let by_key = keys.iter().find_map(|k| o.get(*k)?.as_str());
    by_key.or_else(|| o.values().find_map(|x| x.as_str()))
}

/// `lines` without a fence whose only content was a lifted call: left, it
/// drew as an empty code block in the middle of the answer.
fn without_empty_fences(lines: &[&str]) -> String {
    let mut out: Vec<&str> = Vec::new();
    let (mut i, mut inside) = (0, false);
    while i < lines.len() {
        let t = lines[i].trim();
        let next = (i + 1..lines.len()).find(|&j| !lines[j].trim().is_empty());
        let closes = next.filter(|&j| is_fence(lines[j].trim()));
        match closes {
            Some(j) if !inside && opens_fence(t) => i = j + 1,
            _ => {
                inside ^= opens_fence(t);
                out.push(lines[i]);
                i += 1;
            }
        }
    }
    out.join("\n")
}

/// A line one of the engines writes under an answer, never the agent: the
/// checklist's unfinished steps (`todo::finished`), the spent budget, a call
/// that never ran. The relay keeps its routing line below them (crew-plugin
/// `hopturn::routed_last`).
pub fn is_note(line: &str) -> bool {
    let l = line.trim();
    l.starts_with("checklist: ") || l.starts_with(BUDGET) || l.starts_with(NOT_RUN)
}

#[cfg(test)]
#[path = "neverran_tests.rs"]
mod tests;

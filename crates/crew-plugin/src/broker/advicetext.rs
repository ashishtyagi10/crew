//! The advisor's words (`advise`): the brief it reads, and what of its reply
//! reaches the agent and the pane.
use crate::broker::route::clip;
use crew_hive::clip_middle;
use crew_hive::tools::ToolCall;

/// The heading the advice rides under in the follow-up: whose words they are,
/// and that the agent is not bound by them.
pub(super) const HEAD: &str = "ADVICE (a second model read the failure; you decide):";

/// Lines of advice kept, and chars of each.
const LINES: usize = 3;
const LINE_CAP: usize = 240;
/// Chars of the request, of the failure, and of a call's arguments.
const REQUEST_CAP: usize = 2_000;
const FAILURE_CAP: usize = 4_000;
const ARGS_CAP: usize = 80;
/// Chars of the advice smith's line shows.
const SAID_CAP: usize = 80;

/// Call `n` of the turn, as the advisor's list shows it:
/// `2. sys:run {"cmd": "cargo build"} → failed`.
pub(super) fn call_line(n: usize, call: &ToolCall, ok: bool) -> String {
    let how = if ok { "ok" } else { "failed" };
    let args = clip(&call.args, ARGS_CAP);
    format!("{n}. {} {args} \u{2192} {how}", call.label())
}

/// What the advisor is shown: what it is for, the request, the turn's calls,
/// and the failed command with its output cut to the failure (`failexcerpt`).
/// The request is cut from the middle: a relay's body opens with the project
/// card and standing memory, and the request itself comes last.
pub(super) fn brief(request: &str, calls: &[String], call: &ToolCall, output: &str) -> String {
    let cmd = serde_json::from_str::<serde_json::Value>(&call.args)
        .ok()
        .and_then(|v| v["cmd"].as_str().map(str::to_string))
        .unwrap_or_else(|| call.args.clone());
    format!(
        "You advise a coding agent. A command it ran just failed. You only read: you \
         run nothing, and the agent decides what to do with what you say.\n\n\
         THE REQUEST:\n{}\n\nITS CALLS THIS TURN, IN ORDER:\n{}\n\n\
         THE COMMAND THAT FAILED: {cmd}\nITS OUTPUT, CUT TO THE FAILURE:\n{}\n\n\
         In at most three short lines, say what the failure most likely means and the \
         next step that would fix it, naming the file and line where the output does. \
         Read the error itself; do not send the agent to re-read files the error does \
         not point at. Plain lines: no preamble, no code fences.",
        clip_middle(request, REQUEST_CAP),
        calls.join("\n"),
        clip_middle(&crate::broker::failexcerpt::excerpt(output), FAILURE_CAP)
    )
}

/// The advice in `reply`: its first [`LINES`] lines with words in them, each
/// without its list marker and clipped. Empty when it said nothing.
pub(super) fn advice(reply: &str) -> Vec<String> {
    reply
        .lines()
        .map(|l| {
            let l = l.trim();
            ["- ", "* ", "\u{2022} "]
                .iter()
                .find_map(|m| l.strip_prefix(m))
                .unwrap_or(l)
        })
        .filter(|l| !l.trim().is_empty() && !l.starts_with("```"))
        .take(LINES)
        .map(|l| clip(l, LINE_CAP))
        .collect()
}

/// The follow-up's section for `lines`: [`HEAD`], a dash per line, then a
/// blank line, the shape a steer's section has.
pub(super) fn section(lines: &[String]) -> String {
    let items: Vec<String> = lines.iter().map(|l| format!("- {l}")).collect();
    format!("{HEAD}\n{}\n\n", items.join("\n"))
}

/// Smith's line when advice was given: `advised · <its first line>`.
pub(super) fn said(first: &str) -> String {
    format!("advised \u{b7} {}", clip(first, SAID_CAP))
}

#[cfg(test)]
#[path = "advicetext_tests.rs"]
mod tests;

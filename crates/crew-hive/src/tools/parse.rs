//! Reading a `@tool` call out of a reply that was written for a person.
//!
//! The convention says the call is the reply's final line, and models keep it
//! loosely: they fence the call in a code block, pretty-print the JSON over
//! six lines, add "I'll read the results next." under it, or finish with the
//! relay's `@done` out of habit. The parser read only the last line, so every
//! one of those was taken for the ANSWER: the pane showed the raw `@tool …`
//! text and the tool never ran. This reads the call where the model put it,
//! the way Aider reads a malformed edit block — lenient about the wrapping,
//! strict about what counts as the reply's last word.

use super::around::{ends_the_reply, fence_above};
use super::{JsonDepth, ToolCall};

/// Read the tool call a reply ends with. `None` = the agent answered instead
/// of calling something.
///
/// The call is the LAST line that starts `@tool ` under whatever markdown the
/// model wrapped it in; its JSON runs on to the lines below until it closes;
/// what follows must be the reply's tail (see [`ends_the_reply`]).
pub fn parse_tool_call(reply: &str) -> Option<ToolCall> {
    split_tool_call(reply).map(|(_, call)| call)
}

/// The tool call a reply ends with, and the reply without it.
///
/// Callers need both halves. The text above a call is what the model said
/// while making it ("the bug is in clip(); reading it next"), and a
/// follow-up prompt that drops it leaves the model to work out again why it
/// asked. And a call that is never run must come out of the answer WHOLE:
/// cutting only the last line left a fenced, pretty-printed call as a
/// dangling fence and six lines of JSON. So the cut takes the `@tool` line,
/// its JSON, everything the parser let through below it (the closing fence,
/// a routing line, a short aside) and the fence it opened in; what is left
/// is the text before it, trailing whitespace trimmed.
pub fn split_tool_call(reply: &str) -> Option<(String, ToolCall)> {
    let lines: Vec<&str> = reply.lines().collect();
    let (at, call) = last_call(&lines)?;
    Some((above(&lines, at), call))
}

/// Every tool call a reply ends with, in the order written, and the reply
/// without them.
///
/// An agent that knows it needs three files at the start used to read them in
/// three rounds, each one a whole model call re-sending the prompt; Claude Code
/// and Codex ask for all three in one turn. So the calls may sit on
/// CONSECUTIVE last lines. The last is read as leniently as [`split_tool_call`]
/// reads it; each one above it must be whole where it stands (one line, or
/// JSON that closes) with nothing but blank lines between it and the next. A
/// call with prose under it is one the model was quoting, and stays text.
pub fn split_tool_calls(reply: &str) -> Option<(String, Vec<ToolCall>)> {
    let lines: Vec<&str> = reply.lines().collect();
    let (mut top, last) = last_call(&lines)?;
    let mut calls = vec![last];
    while let Some((at, call)) = call_ending(&lines[..top]) {
        calls.push(call);
        top = at;
    }
    calls.reverse();
    Some((above(&lines, top), calls))
}

/// The reply's last call and the line it starts on, when what follows it
/// leaves it the reply's last word.
fn last_call(lines: &[&str]) -> Option<(usize, ToolCall)> {
    let at = lines.iter().rposition(|l| directive(l).is_some())?;
    let (call, used) = call_at(lines, at)?;
    ends_the_reply(&lines[at + 1 + used..]).then_some((at, call))
}

/// The call that ends `lines` whole, and the line it starts on: only blank
/// lines under it, and its JSON, if it opened any, closed.
fn call_ending(lines: &[&str]) -> Option<(usize, ToolCall)> {
    let end = lines.iter().rposition(|l| !l.trim().is_empty())? + 1;
    let at = lines[..end].iter().rposition(|l| directive(l).is_some())?;
    let (call, used) = call_at(&lines[..end], at)?;
    (at + 1 + used == end && closes(&call.args)).then_some((at, call))
}

/// The call on line `at`, and how many lines under it its JSON took.
fn call_at(lines: &[&str], at: usize) -> Option<(ToolCall, usize)> {
    let rest = directive(lines[at])?;
    let (target, head) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let target = target.trim_matches(['`', '*', '_']);
    let (server, tool) = target.split_once(':')?;
    if server.is_empty() || tool.is_empty() {
        return None;
    }
    let (args, used) = args(head, &lines[at + 1..]);
    let call = ToolCall {
        server: server.to_string(),
        tool: tool.to_string(),
        args,
    };
    Some((call, used))
}

/// The reply above the call on line `at`: the fence the call opened in goes
/// with it, and trailing whitespace is trimmed.
fn above(lines: &[&str], at: usize) -> String {
    let from = fence_above(&lines[..at]).unwrap_or(at);
    lines[..from].join("\n").trim_end().to_string()
}

/// Whether `args` is whole: not JSON at all, or JSON that closes. Args cut off
/// mid-value are let through on the last call, where the tool refuses them and
/// says why; above another call they mean the call was never finished.
fn closes(args: &str) -> bool {
    let t = args.trim_start();
    let mut depth = JsonDepth::default();
    !t.starts_with(['{', '[']) || t.chars().any(|c| depth.push(c))
}

/// What follows `@tool ` on `line`, when `line` is a call: past emphasis, a
/// code span, a list bullet or a quote marker, which models add unbidden.
///
/// Also what follows `CALLED `, spelled exactly so: the exchange log shows
/// every earlier call as `CALLED <tool> <args>` (`exchanges`), and a model
/// taking a steer mid-turn wrote its next call in that shape — it never ran,
/// and the raw line was the answer's end (live probe, 2026-09-30).
fn directive(line: &str) -> Option<&str> {
    let mut s = line.trim_start();
    while let Some(t) = s
        .strip_prefix("- ")
        .or_else(|| s.strip_prefix(['*', '`', '_', '>', ' ', '\t']))
    {
        s = t;
    }
    if let Some(logged) = s.strip_prefix("CALLED ") {
        return Some(logged.trim());
    }
    let head = s.get(..6)?;
    head.eq_ignore_ascii_case("@tool ").then(|| s[6..].trim())
}

/// The call's arguments, and how many lines under the `@tool` line they took.
///
/// JSON is kept exactly as written, newlines and all: the tool parses it
/// either way, and re-serialising would reorder the keys on the card. It may
/// start on the call's line or, bare target above it, on the next one. Args
/// that are not JSON, or JSON that never closes (a reply cut off by its token
/// cap), are the rest of the line, which is what they always were.
fn args(head: &str, after: &[&str]) -> (String, usize) {
    let start = head.trim_start().trim_start_matches('`');
    let json = if start.starts_with(['{', '[']) {
        json(start, after)
    } else if start.is_empty() {
        after
            .first()
            .map(|l| l.trim_start())
            .filter(|l| l.starts_with('{'))
            .and_then(|l| json(l, &after[1..]))
            .map(|(j, used)| (j, used + 1))
    } else {
        None
    };
    json.unwrap_or_else(|| (head.trim().trim_matches('`').to_string(), 0))
}

/// The JSON value `first` opens, read across `more` until it closes, with the
/// number of `more` lines it took. Whatever shares its closing line — a `**`,
/// a backtick — is left behind.
fn json(first: &str, more: &[&str]) -> Option<(String, usize)> {
    let mut depth = JsonDepth::default();
    let mut text = String::new();
    for (used, line) in std::iter::once(first)
        .chain(more.iter().copied())
        .enumerate()
    {
        if used > 0 {
            text.push('\n');
        }
        if let Some((i, c)) = line.char_indices().find(|&(_, c)| depth.push(c)) {
            text.push_str(&line[..i + c.len_utf8()]);
            return Some((text, used));
        }
        text.push_str(line);
    }
    None
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "parsemany_tests.rs"]
mod many_tests;

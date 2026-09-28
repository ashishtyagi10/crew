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

use super::{JsonDepth, ToolCall};

/// Prose lines a call may be followed by and still be a call.
///
/// A model that calls a tool often says one more thing on its way out. A
/// model EXPLAINING the syntax — answering a question about crew, or quoting
/// a call it made earlier — writes a paragraph after it. Past two short lines
/// the call is an example, and running it would act on something the model
/// was only showing.
const TRAILING_PROSE: usize = 2;

/// Longest line that still counts as an aside rather than an explanation.
const SHORT: usize = 160;

/// Phrases that take a call back. A model that writes a call and then "never
/// mind" has changed its mind; running the tool anyway does what it just
/// said not to.
const RETRACTS: [&str; 6] = [
    "never mind",
    "nevermind",
    "scratch that",
    "disregard",
    "on second thought",
    "ignore that",
];

/// Read the tool call a reply ends with. `None` = the agent answered instead
/// of calling something.
///
/// The call is the LAST line that starts `@tool ` under whatever markdown the
/// model wrapped it in; its JSON runs on to the lines below until it closes;
/// what follows must be the reply's tail (see [`ends_the_reply`]).
pub fn parse_tool_call(reply: &str) -> Option<ToolCall> {
    let lines: Vec<&str> = reply.lines().collect();
    let at = lines.iter().rposition(|l| directive(l).is_some())?;
    let rest = directive(lines[at])?;
    let (target, head) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let target = target.trim_matches(['`', '*', '_']);
    let (server, tool) = target.split_once(':')?;
    if server.is_empty() || tool.is_empty() {
        return None;
    }
    let (args, used) = args(head, &lines[at + 1..]);
    ends_the_reply(&lines[at + 1 + used..]).then(|| ToolCall {
        server: server.to_string(),
        tool: tool.to_string(),
        args,
    })
}

/// What follows `@tool ` on `line`, when `line` is a call: past emphasis, a
/// code span, a list bullet or a quote marker, which models add unbidden.
fn directive(line: &str) -> Option<&str> {
    let mut s = line.trim_start();
    while let Some(t) = s
        .strip_prefix("- ")
        .or_else(|| s.strip_prefix(['*', '`', '_', '>', ' ', '\t']))
    {
        s = t;
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

/// Whether what follows a call leaves it the reply's last word: blank lines,
/// the fence it sat in, the relay's routing line, and at most
/// [`TRAILING_PROSE`] short asides that do not take it back.
fn ends_the_reply(after: &[&str]) -> bool {
    let mut prose = 0;
    for line in after {
        let t = line.trim();
        if t.is_empty() || is_fence(t) || is_routing(t) {
            continue;
        }
        let lower = t.to_lowercase();
        if RETRACTS.iter().any(|r| lower.contains(r)) {
            return false;
        }
        prose += 1;
        if prose > TRAILING_PROSE || t.chars().count() > SHORT {
            return false;
        }
    }
    true
}

/// A bare code fence: the one a call was wrapped in closing behind it.
fn is_fence(t: &str) -> bool {
    t.len() >= 3 && t.chars().all(|c| c == '`' || c == '~')
}

/// `@done` / `@next <agent>`, read as tolerantly as the relay's own routing
/// parser reads them. A relay agent ends every reply with one, and a call
/// above it is still the reply's business before routing is.
fn is_routing(t: &str) -> bool {
    let bare = t
        .trim_matches(|c: char| matches!(c, '*' | '`' | '_' | ' ' | '.'))
        .to_ascii_lowercase();
    bare.starts_with("@done") || bare.starts_with("@next")
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;

//! The relay protocol. [`frame`] builds each agent's prompt from the original
//! task, a compact transcript of the conversation so far, and the message for
//! it — so no agent loses the thread. [`parse_routing`] reads the agent's reply:
//! the answer is everything above the final control line, which is `@next
//! <agent>` (hand off) or `@done` (finish). A missing/garbled directive safely
//! ends the thread rather than mis-routing.
use super::Envelope;

/// What the broker does with an agent's reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routing {
    /// Hand off to a named peer with this body (the answer, control line removed).
    Relay { to: String, body: String },
    /// End the thread; the string is the final answer (control line removed).
    Done(String),
}

/// Parse an agent reply into a [`Routing`] decision. The control directive is
/// the last non-empty line; everything above it is the answer body.
pub fn parse_routing(reply: &str) -> Routing {
    let lines: Vec<&str> = reply.lines().collect();
    let mut end = lines.len();
    while end > 0 && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    if end > 0 {
        // Tolerate the markdown/backtick wrappers and stray punctuation real
        // agents put around directives (`**@next codex**`, `` `@done` ``, `@done.`).
        let last = lines[end - 1]
            .trim()
            .trim_matches(|c: char| matches!(c, '*' | '`' | '_' | ' ' | '.'));
        let lower = last.to_ascii_lowercase();
        let body = || lines[..end - 1].join("\n").trim().to_string();
        if lower.starts_with("@next") {
            let arg = last[5..]
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '-')
                .split_whitespace()
                .next()
                .unwrap_or("");
            if !arg.is_empty() {
                return Routing::Relay {
                    to: arg.to_string(),
                    body: body(),
                };
            }
        } else if lower.starts_with("@done") {
            return Routing::Done(body());
        }
    }
    // No control directive: don't guess a recipient — end with the whole reply.
    Routing::Done(reply.trim().to_string())
}

/// Whether `reply` ends with a recognizable control directive (same tolerant
/// matching as [`parse_routing`]). Used to decide whether a repair is needed.
pub fn has_directive(reply: &str) -> bool {
    let mut lines: Vec<&str> = reply.lines().collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    match lines.last() {
        Some(l) => {
            let s = l
                .trim()
                .trim_matches(|c: char| matches!(c, '*' | '`' | '_' | ' ' | '.'))
                .to_ascii_lowercase();
            s.starts_with("@next") || s.starts_with("@done")
        }
        None => false,
    }
}

/// A focused re-prompt for an agent that forgot its control line: show its own
/// reply back and ask only for the missing `@next`/`@done` final line.
pub fn repair_prompt(peers: &[String], prev: &str) -> String {
    let peer_list = if peers.is_empty() {
        "(none)".to_string()
    } else {
        peers.join(", ")
    };
    format!(
        "Your previous reply was missing the required final control line:\n\n\
         {prev}\n\n\
         Reply again with your answer, then make the FINAL line exactly one of:\n\
         - `@next <agent>` to hand off to a peer (only from: {peer_list})\n\
         - `@done` if the task is complete.",
    )
}

/// Cap on the task text interpolated into every hop's prompt (see [`frame`]).
/// The full `task` is repeated on EVERY hop of a relay, so an unbounded task
/// costs its full length again and again as a thread grows; this is generous
/// enough that normal tasks never notice, and only pathologically long ones
/// get clipped (with a trailing ellipsis via [`clip`]).
const TASK_CAP: usize = 4000;

/// Build the prompt for the agent named by `env.to`. The invariant content
/// (identity, task, the `@next`/`@done` protocol) comes first so repeated calls
/// to the same agent in a thread share a cacheable prefix; the variable parts
/// (transcript, then the current message — already a normalized reply, never raw
/// CLI chatter) come last, with the message most salient.
pub fn frame(env: &Envelope, peers: &[String], task: &str, transcript: &str) -> String {
    let peer_list = if peers.is_empty() {
        "(none)".to_string()
    } else {
        peers.join(", ")
    };
    let convo = if transcript.trim().is_empty() {
        "(you are first — no replies yet)".to_string()
    } else {
        transcript.to_string()
    };
    let task = clip(task.trim(), TASK_CAP);
    compact_ws(&format!(
        "You are \"{me}\", a CLI coding agent working with peers: {peers}.\n\n\
         TASK:\n{task}\n\n\
         HOW TO REPLY: answer concisely, then make the FINAL line exactly one of:\n\
         - `@next <agent>` to hand the conversation to a peer (only from: {peers})\n\
         - `@done` if the task is complete and no further reply is needed.\n\n\
         CONVERSATION SO FAR:\n{convo}\n\n\
         MESSAGE FOR YOU FROM \"{from}\":\n{body}",
        me = env.to,
        peers = peer_list,
        task = task,
        convo = convo,
        from = env.from,
        body = env.body,
    ))
}

/// Collapse 3+ consecutive newlines into 2 and strip trailing spaces/tabs from
/// each line — removes prompt-padding tokens without changing meaning.
pub(crate) fn compact_ws(s: &str) -> String {
    let stripped: Vec<&str> = s.lines().map(|l| l.trim_end_matches([' ', '\t'])).collect();
    let joined = stripped.join("\n");
    let mut out = String::with_capacity(joined.len());
    let mut newline_run = 0usize;
    for ch in joined.chars() {
        if ch == '\n' {
            newline_run += 1;
            if newline_run <= 2 {
                out.push(ch);
            }
        } else {
            newline_run = 0;
            out.push(ch);
        }
    }
    out
}

/// Flatten whitespace and clip a transcript body to at most `max` chars (adding
/// `…`), keeping the conversation summary compact so prompt size — and cost —
/// stays bounded as a thread grows. The immediate recipient still sees the full
/// message via the envelope body; only the historical summary is clipped.
pub(crate) fn clip(s: &str, max: usize) -> String {
    let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        flat
    } else {
        format!("{}…", flat.chars().take(max).collect::<String>())
    }
}

#[cfg(test)]
#[path = "route_tests.rs"]
mod tests;

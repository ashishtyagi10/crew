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

/// Build the prompt for the agent named by `env.to`: the task, the transcript so
/// far, and the message addressed to it (already a normalized reply, never raw
/// CLI chatter), plus the `@next`/`@done` protocol.
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
    format!(
        "You are \"{me}\", a CLI coding agent working with peers: {peers}.\n\n\
         TASK:\n{task}\n\n\
         CONVERSATION SO FAR:\n{convo}\n\n\
         MESSAGE FOR YOU FROM \"{from}\":\n{body}\n\n\
         Answer concisely. Then make the FINAL line of your reply exactly one of:\n\
         - `@next <agent>` to hand the conversation to a peer (only from: {peers})\n\
         - `@done` if the task is complete and no further reply is needed.",
        me = env.to,
        peers = peer_list,
        task = task,
        convo = convo,
        from = env.from,
        body = env.body,
    )
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

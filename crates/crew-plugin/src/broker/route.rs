//! The relay protocol. The broker [`frame`]s each envelope into a prompt that
//! tells the agent who it is, who its peers are, and how to hand off or finish.
//! [`parse_routing`] reads the agent's reply back into a routing decision:
//! relay to a peer, end the thread, or reply to the sender.
use super::Envelope;

/// What the broker does with an agent's reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routing {
    /// Hand off to a named peer with this body.
    Relay { to: String, body: String },
    /// End the thread; the string is the final answer (may be empty).
    Done(String),
    /// Plain reply back to whoever sent the message.
    Reply(String),
}

/// Parse an agent reply into a [`Routing`] decision.
///
/// - A first line `TO <peer>: <msg>` (case-insensitive) relays to `<peer>`.
/// - A reply of `DONE` or `DONE: <answer>` ends the thread.
/// - Anything else is a plain reply to the sender.
pub fn parse_routing(reply: &str) -> Routing {
    let trimmed = reply.trim();
    let first = trimmed.lines().next().unwrap_or("").trim();
    let lower = first.to_ascii_lowercase();

    if let Some(rest) = lower.strip_prefix("to ") {
        let head_len = first.len() - rest.len();
        let head = &first[head_len..]; // original-case remainder of line 1
        let (to, inline) = match head.split_once(':') {
            Some((a, m)) => (a.trim(), m.trim()),
            None => (head.trim(), ""),
        };
        let tail: String = trimmed.lines().skip(1).collect::<Vec<_>>().join("\n");
        let tail = tail.trim();
        let body = match (inline.is_empty(), tail.is_empty()) {
            (false, false) => format!("{inline}\n{tail}"),
            (false, true) => inline.to_string(),
            (true, _) => tail.to_string(),
        };
        return Routing::Relay {
            to: to.to_string(),
            body,
        };
    }

    if lower == "done" || lower.starts_with("done:") || lower.starts_with("done ") {
        let answer = trimmed[4..].trim_start_matches(':').trim();
        return Routing::Done(answer.to_string());
    }

    Routing::Reply(trimmed.to_string())
}

/// Build the prompt sent to the agent named by `env.to`. The body is the only
/// content from another agent, and it is already a normalized reply — never raw
/// CLI chatter.
pub fn frame(env: &Envelope, peers: &[String]) -> String {
    let peer_list = if peers.is_empty() {
        "(none)".to_string()
    } else {
        peers.join(", ")
    };
    format!(
        "You are the coding agent \"{me}\" in a multi-agent relay.\n\
         Peers you may hand off to: {peers}.\n\
         Message from \"{from}\":\n{body}\n\n\
         Reply concisely. To hand a peer the next step, begin your reply with a \
         line `TO <peer>: <message>`. If the task is complete and no further \
         reply is needed, reply with exactly `DONE` (optionally `DONE: <final \
         answer>`). Otherwise just answer and it goes back to \"{from}\".",
        me = env.to,
        peers = peer_list,
        from = env.from,
        body = env.body,
    )
}

#[cfg(test)]
#[path = "route_tests.rs"]
mod tests;

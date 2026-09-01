//! Reading what a session said, and what a person said back.
//!
//! Split out of [`super::task`] for size, and the two halves are genuinely different jobs: one
//! decides which broker events are worth forwarding to a phone, the other decides whether a
//! human just said yes. Neither knows anything about sessions.
use crew_plugin::PluginEvent;

/// What crew says when it has taken an answer.
pub(crate) const ALLOWED: &str = "approved \u{2014} carrying on";
pub(crate) const REFUSED: &str = "refused \u{2014} I will not do it";
/// What it says when the answer was neither. Deliberately does NOT guess: the whole point of
/// asking is that somebody meant to say yes or no, and reading "maybe later" as either is worse
/// than asking twice.
pub(crate) const UNCLEAR: &str = "I need a yes or a no on that one.";

/// Something from a session worth sending on.
#[derive(Debug, PartialEq)]
pub(crate) enum Emitted {
    /// A finished reply.
    Reply(String),
    /// A question a human has to answer before the agent can continue.
    Ask { id: String, question: String },
}

/// What one broker output line means to the channel, if anything.
pub(crate) fn emitted(line: &str) -> Option<Emitted> {
    match serde_json::from_str::<PluginEvent>(line).ok()? {
        PluginEvent::Message { text, .. } if !text.trim().is_empty() => Some(Emitted::Reply(text)),
        PluginEvent::Approval { id, question, .. } => Some(Emitted::Ask { id, question }),
        _ => None,
    }
}

/// Read a yes or a no out of what somebody typed. `None` for anything else — including an empty
/// message, and including words like "maybe": an approval is exactly the place not to guess.
pub(crate) fn parse_answer(text: &str) -> Option<bool> {
    match text
        .trim()
        .to_lowercase()
        .trim_end_matches(['.', '!'])
        .trim()
    {
        "y" | "yes" | "ok" | "okay" | "sure" | "approve" | "approved" | "go" | "go ahead"
        | "do it" | "allow" => Some(true),
        "n" | "no" | "nope" | "deny" | "denied" | "refuse" | "stop" | "cancel" | "don't"
        | "dont" => Some(false),
        _ => None,
    }
}

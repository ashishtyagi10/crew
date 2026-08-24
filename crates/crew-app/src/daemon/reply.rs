//! What the resident says back when somebody messages it.
//!
//! Deliberately small. A channel that answers three honest questions is worth more than one that
//! pretends to be an agent and is not: the moment a message can reach a session this becomes the
//! fallback rather than the whole vocabulary, and the wrong version of it is one that quietly
//! swallows anything it does not understand.
use super::session::Card;

/// What the resident knows about itself when it answers.
pub(crate) struct Snapshot {
    pub version: String,
    pub uptime_s: u64,
    pub sessions: Vec<Card>,
}

/// The reply to one inbound message, or `None` when it is not one of the resident's own
/// questions — those go to an agent session instead of being answered here.
pub(crate) fn respond(text: &str, snap: &Snapshot) -> Option<String> {
    let word = text
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();
    Some(match word.as_str() {
        "help" | "?" | "/help" | "/start" => HELP.to_string(),
        "status" | "/status" => format!(
            "crew {} \u{2014} up {}, {} session(s)",
            snap.version,
            human_uptime(snap.uptime_s),
            snap.sessions.len()
        ),
        "sessions" | "/sessions" => {
            if snap.sessions.is_empty() {
                return Some("no sessions".to_string());
            }
            snap.sessions
                .iter()
                .map(|c| {
                    format!(
                        "{}  {}  {}",
                        c.id,
                        if c.alive { "running" } else { "dead" },
                        c.label
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        // Anything else is work for an agent, not a question about the resident.
        _ => return None,
    })
}

/// The vocabulary, named once so a command cannot quietly stop being mentioned in the help text.
#[cfg(test)]
const KNOWN: &[&str] = &["help", "status", "sessions"];

const HELP: &str = "crew, at your service.\n\
     Anything you say that is not one of these becomes a task for an agent:\n\
     help \u{2014} this\n\
     status \u{2014} version, uptime, session count\n\
     sessions \u{2014} the agent sessions I am holding";

/// Uptime a person can read at a glance.
pub(crate) fn human_uptime(secs: u64) -> String {
    match secs {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m", s / 60),
        s if s < 86_400 => format!("{}h{}m", s / 3600, (s % 3600) / 60),
        s => format!("{}d{}h", s / 86_400, (s % 86_400) / 3600),
    }
}

#[cfg(test)]
#[path = "reply_tests.rs"]
mod tests;

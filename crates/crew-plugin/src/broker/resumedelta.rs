//! What a resumed Claude Code session is sent: only what it has not read.
//!
//! A follow-up continued with `--resume` (`clauderesume`) was still sent the
//! whole frame: the opening, the tools hint, the reply protocol, the project
//! card, the skills list and a recap of the turns before — every one of them
//! already in the session it continues, and the recap a second, clipped copy
//! of the exchange the session holds verbatim. Measured live (2026-11-04,
//! Claude Code 2.1.318): a 5.8 KB prompt for a one-line follow-up, and the
//! session's context grew by ~2.5k tokens a message for it.
//!
//! The frame is cut by paragraph. A paragraph the session was already sent is
//! dropped, so a changed one (the card after a commit, a skill picked for
//! this message) still goes; the thread's recap is dropped whole; the newest
//! message is never cut, even where it repeats an earlier one word for word.
use std::collections::HashSet;

use super::thread::HEAD;

/// The paragraphs a session has been sent, trimmed.
pub(super) type Seen = HashSet<String>;

/// `thread::with_context`'s seam between the recap and the message.
const NOW: &str = "\n\nNow:\n";

fn paragraphs(text: &str) -> impl Iterator<Item = &str> {
    text.split("\n\n").map(str::trim).filter(|p| !p.is_empty())
}

/// Every paragraph of `body`, added to what the session had already seen.
pub(super) fn seen_with(mut seen: Seen, body: &str) -> Seen {
    seen.extend(paragraphs(body).map(str::to_owned));
    seen
}

/// `body` as a session that has read `seen` needs it: the paragraphs it has
/// not read, then the message whole.
pub(super) fn news(body: &str, seen: &Seen) -> String {
    // The message: after the recap when there is one (glued to the `TASK:`
    // line the recap sat under), else the last paragraph.
    let (head, glue, ask) = match recap(body) {
        Some((start, end)) => (&body[..start], "\n", &body[end..]),
        None => match body.trim_end().rfind("\n\n") {
            Some(cut) => (&body[..cut], "\n\n", body[cut..].trim_start()),
            None => return body.to_owned(),
        },
    };
    let kept: Vec<&str> = paragraphs(head).filter(|p| !seen.contains(*p)).collect();
    match kept.is_empty() {
        true => ask.to_owned(),
        false => format!("{}{glue}{ask}", kept.join("\n\n")),
    }
}

/// Where the thread's recap starts and where the message after it starts.
fn recap(body: &str) -> Option<(usize, usize)> {
    let start = body.find(HEAD)?;
    let end = start + body[start..].find(NOW)? + NOW.len();
    Some((start, end))
}

#[cfg(test)]
#[path = "resumedelta_tests.rs"]
mod tests;

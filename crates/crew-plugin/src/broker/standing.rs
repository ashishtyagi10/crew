//! A standing preference is kept the first time it is said.
//!
//! WHY: "you should be talking like Agent Smith" steered one turn and was
//! gone. It survived only as a recalled turn, quoted back when a later
//! message happened to share a word with it, while `#<note>` (the door that
//! makes a preference standing, `memory`) had to be known about and typed.
//! Six weeks on there was no memory file at all (live, 2026-11-14). A user
//! says how they want to be worked with once, in their own words, and
//! expects it to hold.
//!
//! Two sources, as for a correction (`correction`): the router's optional
//! `REMEMBER:` line (`intent::hints`), so the model decides what is a
//! lasting preference and what is this task's instruction; where no router
//! read the message, a short list of openings. Either way the note lands in
//! `./.crew/memory.md`, the file `#<note>` writes and every task carries,
//! and the pane says so in one line, so nothing is kept unseen.
use super::thread::clip_chars;

/// Chars of the user's message kept when the openings decide.
const SAID_KEPT: usize = 200;

/// Openings that say "from here on", not "for this task". Start of the
/// message only, and never a question: a wrong note rides in front of every
/// later task until someone deletes it.
const OPENINGS: [&str; 10] = [
    "from now on",
    "going forward",
    "in future,",
    "in the future,",
    "always ",
    "never ",
    "you should always ",
    "you should never ",
    "you should be ",
    "please always ",
];

/// The note the openings find in `task`: the message itself, clipped.
pub(crate) fn backstop(task: &str) -> Option<String> {
    let said = task.trim();
    let t = said.to_lowercase();
    let lasting = OPENINGS.iter().any(|o| t.starts_with(o))
        && !t.starts_with("never mind")
        && !t.ends_with('?');
    lasting.then(|| clip_chars(said, SAID_KEPT))
}

/// Keep `task`'s standing preference when it states one, and return the
/// line to say. `clause` is the router's verdict, read like a correction's:
/// `Some(Some(_))` it said REMEMBER, `Some(None)` it read the message and
/// said nothing (trusted), `None` no router read it, so [`backstop`] does.
/// A note the file already holds is not written twice and nothing is said.
pub(crate) fn note(clause: Option<Option<&str>>, task: &str) -> Option<String> {
    let what = match clause {
        Some(Some(c)) => c.trim().to_string(),
        Some(None) => return None,
        None => backstop(task)?,
    };
    let what = what.replace('\n', " ");
    let path = super::memory::base_dir().join(".crew").join("memory.md");
    let held = std::fs::read_to_string(path).unwrap_or_default();
    let known = held.lines().any(|l| {
        l.trim_start_matches("- ")
            .trim()
            .eq_ignore_ascii_case(&what)
    });
    if what.is_empty() || known {
        return None;
    }
    let said = super::memory::remember(&what);
    Some(match said.starts_with("remembered") {
        true => format!(
            "remembered for every task from now on: \u{201c}{what}\u{201d} \u{2014} \
             .crew/memory.md (edit the file to change it)"
        ),
        false => said, // the write failed, and says why
    })
}

#[cfg(test)]
#[path = "standing_tests.rs"]
mod tests;

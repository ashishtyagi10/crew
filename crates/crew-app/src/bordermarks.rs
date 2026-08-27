//! Whether pane cards mark what they know on their borders.
//!
//! Two markings answer to this: the ticks where each command began
//! ([`crate::cmdspan`]) and the bars beside error lines
//! ([`crate::errscan`]). They are on by default — a grid of panes saying
//! where the failures are without being read is most of their value — but
//! they are crew drawing on its own chrome about someone else's output, and a
//! plain frame is a reasonable thing to want.
//!
//! A lock-free flag, like every other look switch: it is read once per pane
//! per frame and set from three places (config load, `/marks`, the settings
//! form), so a global beats threading a bool through the frame builder.
use std::sync::atomic::{AtomicBool, Ordering};

static ON: AtomicBool = AtomicBool::new(true);

/// Whether the border markings are drawn.
pub(crate) fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

pub(crate) fn set(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

/// Parse a `/marks` argument. `None` for anything that is not an answer —
/// the caller reports the current state rather than guessing at one.
pub(crate) fn parse(arg: &str) -> Option<bool> {
    match arg.trim().to_ascii_lowercase().as_str() {
        "on" | "yes" | "true" => Some(true),
        "off" | "no" | "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_switch_starts_on_because_the_marks_are_the_default() {
        // Set explicitly rather than assumed: another test may have moved it.
        set(true);
        assert!(on());
        set(false);
        assert!(!on());
        set(true);
    }

    #[test]
    fn every_spelling_of_an_answer_parses_and_nothing_else_does() {
        for yes in ["on", "ON", " yes ", "true"] {
            assert_eq!(parse(yes), Some(true), "{yes}");
        }
        for no in ["off", "No", "false"] {
            assert_eq!(parse(no), Some(false), "{no}");
        }
        for neither in ["", "maybe", "1", "auto"] {
            assert_eq!(parse(neither), None, "{neither}");
        }
    }
}

//! Whether the file viewer shows the characters that say something without
//! printing anything — tabs, trailing spaces, and the carriage return a CRLF
//! file leaves at the end of every line.
//!
//! Off by default: revealing them is a *diagnostic* view, and the three marks
//! it draws are noise in every file that has nothing wrong with it. It is on
//! when you already suspect a tab where spaces were meant, whitespace nobody
//! can see at the end of a line, or a line ending that makes a shell script
//! fail with a message about a command that does not exist.
//!
//! Note what does NOT answer to this switch: tabs are always *expanded*.
//! That is not a reveal, it is the difference between drawing a file's
//! indentation and dropping it — see [`crate::viewpane::whitespace`].
//!
//! A lock-free flag, like every other look switch: read while a viewer builds
//! its lines and set from three places (config load, `/invisibles`, the
//! settings form).
use std::sync::atomic::{AtomicBool, Ordering};

static ON: AtomicBool = AtomicBool::new(false);

/// Whether the viewer reveals its invisibles.
pub(crate) fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

pub(crate) fn set(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_switch_starts_off_because_a_clean_file_has_nothing_to_show() {
        // Set explicitly rather than assumed: another test may have moved it.
        set(false);
        assert!(!on());
        set(true);
        assert!(on());
        set(false);
    }
}

//! Taking it back.
//!
//! An editor you cannot undo in is one nobody will type into twice, and with
//! the source as the buffer undo is exact rather than approximate: a change is
//! a byte range, the text that was there and the text that replaced it, so
//! reverting one restores the file to the byte it held before — not to a
//! re-serialization of a tree that happened to mean the same thing.
//!
//! Keystrokes are **coalesced** into words, because undoing a sentence one
//! letter at a time is the same as having no undo. A run breaks where a person
//! would expect it to: at a newline, at a space, when the caret is moved by
//! hand, and when the direction changes from typing to deleting.
/// One reversible change: `removed` was at `at`, and `inserted` is there now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Change {
    pub at: u32,
    pub removed: String,
    pub inserted: String,
    /// Where the caret was before this change — where it belongs again after
    /// the change is undone.
    pub caret: u32,
}

impl Change {
    /// The same change, the other way round.
    fn inverse(&self) -> Change {
        Change {
            at: self.at,
            removed: self.inserted.clone(),
            inserted: self.removed.clone(),
            caret: self.at + self.removed.len() as u32,
        }
    }
}

/// How many changes are kept. A long editing session is thousands of
/// keystrokes and a few hundred *changes* once they are coalesced.
const KEEP: usize = 500;

#[derive(Debug, Default)]
pub(crate) struct History {
    done: Vec<Change>,
    undone: Vec<Change>,
    /// Set when something other than typing happened — a caret move, a save —
    /// so the next keystroke starts a new run rather than joining the last.
    broken: bool,
}

impl History {
    /// Record a change, merging it into the previous one when the two are one
    /// continuous run of typing (or of deleting).
    pub(crate) fn record(&mut self, c: Change) {
        // Anything new invalidates the redo stack: you cannot go forward down
        // a road you have just left.
        self.undone.clear();
        let merged = match self.done.last_mut() {
            Some(last) if !self.broken => merge(last, &c),
            _ => false,
        };
        self.broken = false;
        if merged {
            return;
        }
        self.done.push(c);
        let over = self.done.len().saturating_sub(KEEP);
        self.done.drain(..over);
    }

    /// End the current run: the next keystroke will be its own change.
    pub(crate) fn breaks(&mut self) {
        self.broken = true;
    }

    /// The change to apply to undo, if there is one.
    pub(crate) fn undo(&mut self) -> Option<Change> {
        let c = self.done.pop()?;
        self.undone.push(c.clone());
        self.broken = true;
        Some(c.inverse())
    }

    /// The change to apply to redo.
    pub(crate) fn redo(&mut self) -> Option<Change> {
        let c = self.undone.pop()?;
        self.done.push(c.clone());
        self.broken = true;
        Some(c)
    }
}

/// Whether `next` continues `last` — and if so, extend `last` to cover both.
///
/// Two runs join: typing forward (each character at the end of the last), and
/// backspacing (each deletion ending where the last began). A newline or a
/// space closes a run, so undo gives back a word at a time rather than a
/// paragraph.
fn merge(last: &mut Change, next: &Change) -> bool {
    let typing = next.removed.is_empty() && last.removed.is_empty();
    let deleting = next.inserted.is_empty() && last.inserted.is_empty();
    if typing && last.at + last.inserted.len() as u32 == next.at {
        if breaks_run(&last.inserted) {
            return false;
        }
        last.inserted.push_str(&next.inserted);
        return true;
    }
    if deleting && next.at + next.removed.len() as u32 == last.at {
        // Symmetric with typing: a run ends when a break character is
        // CONSUMED. Deletions prepend, so the thing deleted most recently is
        // at the FRONT of what the run has taken — checking the incoming
        // character instead let a backspace over a newline join the letters
        // before it into one change, and undoing gave back a newline short.
        if starts_run_break(&last.removed) {
            return false;
        }
        // Deleting runs backwards: the change starts earlier each time.
        last.at = next.at;
        let mut text = next.removed.clone();
        text.push_str(&last.removed);
        last.removed = text;
        return true;
    }
    false
}

/// Whether text just typed ends the run it is part of.
fn breaks_run(text: &str) -> bool {
    text.ends_with('\n') || text.ends_with(' ')
}

/// The same question for a run of deletions, which grows from its front.
fn starts_run_break(text: &str) -> bool {
    text.starts_with('\n') || text.starts_with(' ')
}

#[cfg(test)]
#[path = "undo_tests.rs"]
mod tests;

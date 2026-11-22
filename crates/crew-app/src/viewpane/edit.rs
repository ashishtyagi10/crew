//! Typing into the render.
//!
//! The caret is a byte of the file ([`super::caret`]), so an edit is a splice
//! at that byte and nothing else: the document is re-parsed, re-wrapped, and
//! the caret found again at the byte it moved to. Untouched bytes do not
//! merely *tend* to stay put — nothing in this file can move one, which is
//! what makes the eventual diff minimal by construction rather than by a
//! serializer's promise.
//!
//! What that buys, in the order it matters:
//!
//! * A save is `write(text)`. There is no serializer, so there is no marker
//!   convention to sniff, no `*` bullets turning into `-`, no setext heading
//!   rewritten as ATX, no paragraph re-wrapped — the hostile diff every naive
//!   round-trip produces cannot be produced here.
//! * A character crew's renderer does not understand still round-trips: it is
//!   still in the file, and the file is what is written.
//! * The cost is that some edits are spelled in markdown rather than in the
//!   document model — Enter has to know that a blank line is what separates
//!   two paragraphs, which is the one piece of that spelling this slice needs.
use super::{LoadState, ViewPane};

impl ViewPane {
    /// The document's source, to read.
    pub(crate) fn source_str(&self) -> Option<&str> {
        match &self.state {
            LoadState::Ready { loaded, .. } => Some(&loaded.text),
            _ => None,
        }
    }

    /// How many bytes the document has.
    pub(crate) fn source_len(&self) -> Option<u32> {
        self.source_str().map(|s| s.len() as u32)
    }

    /// Replace `[from, to)` with `text`, recording it for undo.
    pub(crate) fn replace_range(&mut self, from: u32, to: u32, text: &str, cols: u16, rows: u16) {
        let from = from as usize;
        self.splice(from, (to as usize).saturating_sub(from), text, cols, rows);
    }

    /// The document's source, when there is one to edit.
    fn source(&mut self) -> Option<&mut String> {
        match &mut self.state {
            LoadState::Ready { loaded, .. } => Some(&mut loaded.text),
            _ => None,
        }
    }

    /// Insert `text` at the caret, and leave the caret after it. Typing with
    /// a selection replaces it, as it does everywhere else.
    pub(crate) fn insert(&mut self, text: &str, cols: u16, rows: u16) {
        self.delete_selection(cols, rows);
        let Some(at) = self.caret_at else { return };
        self.splice(at as usize, 0, text, cols, rows);
    }

    /// The one mutation. Replace `remove` bytes at `at` with `text`, record
    /// it so it can be taken back, and put the caret after what was written.
    fn splice(&mut self, at: usize, remove: usize, text: &str, cols: u16, rows: u16) {
        let caret = self.caret_at.unwrap_or(at as u32);
        let Some(src) = self.source() else { return };
        let at = at.min(src.len());
        let end = (at + remove).min(src.len());
        if !src.is_char_boundary(at) || !src.is_char_boundary(end) {
            return;
        }
        let removed = src[at..end].to_string();
        src.replace_range(at..end, text);
        self.history.record(super::undo::Change {
            at: at as u32,
            removed,
            inserted: text.to_string(),
            caret,
        });
        self.after_edit(at + text.len(), cols, rows);
    }

    /// Apply a change from the history, without recording it again.
    fn apply(&mut self, c: super::undo::Change, cols: u16, rows: u16) {
        let Some(src) = self.source() else { return };
        let at = (c.at as usize).min(src.len());
        let end = (at + c.removed.len()).min(src.len());
        if !src.is_char_boundary(at) || !src.is_char_boundary(end) {
            return;
        }
        src.replace_range(at..end, &c.inserted);
        self.after_edit(c.caret as usize, cols, rows);
    }

    /// Take back the last run of typing. `false` when there is nothing left.
    pub(crate) fn undo(&mut self, cols: u16, rows: u16) -> bool {
        let Some(c) = self.history.undo() else {
            return false;
        };
        self.apply(c, cols, rows);
        true
    }

    /// Put back what undo took.
    pub(crate) fn redo(&mut self, cols: u16, rows: u16) -> bool {
        let Some(c) = self.history.redo() else {
            return false;
        };
        self.apply(c, cols, rows);
        true
    }

    /// Delete the character AT the caret (forward delete), leaving it put.
    pub(crate) fn delete(&mut self, cols: u16, rows: u16) {
        if self.delete_selection(cols, rows) {
            return;
        }
        let Some(at) = self.caret_at else { return };
        let Some(src) = self.source() else { return };
        let at = (at as usize).min(src.len());
        let Some(next) = src[at..].chars().next() else {
            return;
        };
        self.splice(at, next.len_utf8(), "", cols, rows);
    }

    /// Delete the character before the caret, and leave the caret where it
    /// began. At the very start of the document there is nothing to delete
    /// and nothing happens — including no scroll, which is the thing an
    /// editor that "does nothing" usually still gets wrong.
    pub(crate) fn backspace(&mut self, cols: u16, rows: u16) {
        if self.delete_selection(cols, rows) {
            return;
        }
        let Some(at) = self.caret_at else { return };
        let Some(src) = self.source() else { return };
        let at = (at as usize).min(src.len());
        let Some(prev) = src[..at].chars().next_back() else {
            return;
        };
        let from = at - prev.len_utf8();
        self.splice(from, prev.len_utf8(), "", cols, rows);
    }

    /// Enter: end this block and start another of the same kind.
    ///
    /// This is the one place the source model has to know some markdown. A
    /// single newline inside a paragraph is a *soft* break — CommonMark joins
    /// the two sides with a space — so pressing Enter in prose and getting one
    /// would look like nothing happened at all. A paragraph therefore needs a
    /// blank line, and a list item needs the marker that makes the next line
    /// another item rather than a paragraph interrupting the list.
    pub(crate) fn newline(&mut self, cols: u16, rows: u16) {
        let Some(at) = self.caret_at else { return };
        let Some(src) = self.source() else { return };
        let at = (at as usize).min(src.len());
        let line = src[..at].rfind('\n').map_or(0, |i| i + 1);
        let insert = match super::enterline::continuation(&src[line..at]) {
            _ if super::enterline::in_code(src, at, line) => {
                let indent = src[line..at].len() - src[line..at].trim_start().len();
                format!("\n{}", &src[line..line + indent])
            }
            Some(prefix) => format!("\n{prefix}"),
            None => "\n\n".to_string(),
        };
        self.splice(at, 0, &insert, cols, rows);
    }

    /// Re-read the document at `offset` and put the caret there.
    pub(crate) fn after_edit(&mut self, offset: usize, cols: u16, rows: u16) {
        self.dirty = true;
        self.anchor = None;
        // The layout cache is keyed by width and theme, not by the text — so
        // an edit has to throw it away explicitly, or the pane would draw the
        // document as it was before the keystroke.
        self.cache.replace(None);
        self.caret_at = Some(offset as u32);
        self.relayout_caret(cols, rows);
        // A caret whose byte no longer resolves (an edit that removed the
        // last place to stand) still has to be somewhere.
        if self.caret.is_none() {
            self.start_editing(cols);
        }
        self.clamp_scroll(cols, rows);
    }

    /// Write the document back. The bytes are the ones that were read, with
    /// the edits spliced in — so `git diff` shows what was typed and nothing
    /// else.
    pub(crate) fn save(&mut self) -> std::io::Result<()> {
        let path = self.path.clone();
        let Some(src) = self.source() else {
            return Ok(());
        };
        let text = src.clone();
        std::fs::write(path, text)?;
        self.dirty = false;
        // A save is a place you might come back to: the next keystroke starts
        // its own change rather than joining the run that ended here.
        self.history.breaks();
        Ok(())
    }
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;

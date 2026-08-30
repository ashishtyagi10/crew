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
    /// The document's source, when there is one to edit.
    fn source(&mut self) -> Option<&mut String> {
        match &mut self.state {
            LoadState::Ready { loaded, .. } => Some(&mut loaded.text),
            _ => None,
        }
    }

    /// Insert `text` at the caret, and leave the caret after it.
    pub(crate) fn insert(&mut self, text: &str, cols: u16, rows: u16) {
        let Some(at) = self.caret_at else { return };
        let Some(src) = self.source() else { return };
        let at = (at as usize).min(src.len());
        if !src.is_char_boundary(at) {
            return;
        }
        src.insert_str(at, text);
        self.after_edit(at + text.len(), cols, rows);
    }

    /// Delete the character before the caret, and leave the caret where it
    /// began. At the very start of the document there is nothing to delete
    /// and nothing happens — including no scroll, which is the thing an
    /// editor that "does nothing" usually still gets wrong.
    pub(crate) fn backspace(&mut self, cols: u16, rows: u16) {
        let Some(at) = self.caret_at else { return };
        let Some(src) = self.source() else { return };
        let at = (at as usize).min(src.len());
        let Some(prev) = src[..at].chars().next_back() else {
            return;
        };
        let from = at - prev.len_utf8();
        src.replace_range(from..at, "");
        self.after_edit(from, cols, rows);
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
        let insert = match continuation(&src[line..at]) {
            Some(prefix) => format!("\n{prefix}"),
            None => "\n\n".to_string(),
        };
        src.insert_str(at, &insert);
        self.after_edit(at + insert.len(), cols, rows);
    }

    /// Re-read the document at `offset` and put the caret there.
    fn after_edit(&mut self, offset: usize, cols: u16, rows: u16) {
        self.dirty = true;
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
        Ok(())
    }
}

/// The markdown a new line has to start with to continue the block the caret
/// is in, or `None` for prose (which needs a blank line instead).
///
/// Read off the SOURCE line the caret is on — the one place the markers are
/// still visible, which is exactly why the buffer is the source.
fn continuation(line: &str) -> Option<String> {
    let indent: String = line
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let rest = &line[indent.len()..];
    // A quote inside a list, a list inside a quote: take the markers in the
    // order they were written.
    if let Some(after) = rest.strip_prefix("> ") {
        let inner = continuation(after).unwrap_or_default();
        return Some(format!("{indent}> {inner}"));
    }
    for m in ["- ", "* ", "+ "] {
        if rest.starts_with(m) {
            return Some(format!("{indent}{m}"));
        }
    }
    // `1. ` / `12) ` — the next item is numbered one higher, because a list
    // whose every item says `1.` renders correctly and reads as a mistake in
    // the file.
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if !digits.is_empty() {
        let after = &rest[digits.len()..];
        if let Some(sep) = [". ", ") "].iter().find(|s| after.starts_with(**s)) {
            let n: u32 = digits.parse().unwrap_or(1);
            return Some(format!("{indent}{}{sep}", n + 1));
        }
    }
    None
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;

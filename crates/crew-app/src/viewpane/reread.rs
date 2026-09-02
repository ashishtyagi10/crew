//! Re-reading a document that is being EDITED. The viewer's `r` reloads a
//! file and keeps the pane in place; an editor has more to keep straight —
//! the unsaved edits it is about to lose, the undo history of text that
//! will not exist any more, and the caret, whose byte survives if the new
//! text still has it.
use super::ViewPane;

impl ViewPane {
    /// Throw away the edits and re-read the file. The caret keeps its byte;
    /// the load landing re-finds it in the new text, or the nearest place.
    pub(crate) fn reread(&mut self) {
        self.dirty = false;
        self.history = Default::default();
        self.anchor = None;
        self.reload();
    }
}

#[cfg(test)]
#[path = "reread_tests.rs"]
mod tests;

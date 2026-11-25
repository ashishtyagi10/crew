//! What an editing key does to the document, for both places a document is edited: its own
//! window ([`crate::docwin`]) and a viewer pane in the grid. Classifying the key is
//! `docwin::keys`; this applies it, and says what is left for the host — the clipboard, the
//! disk and the status line are the app's, not the pane's.
use super::ViewPane;
use crate::docwin::keys::Edit;

/// What the host still has to do after an edit was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EditDone {
    /// Applied; redraw.
    Drawn,
    Save,
    Reload,
    /// Put this on the clipboard.
    Copy(String),
    /// Read the clipboard and insert it.
    Paste,
    /// Bold or italic across a block boundary — refused, and the host says why.
    Refused,
    /// Cmd+K: the window's URL field. A pane has none; the caret's block shows the URL anyway.
    Link,
}

impl Edit {
    /// Whether this edit changes the text — what re-arms an "unsaved changes" warning.
    pub(crate) fn changes_text(&self) -> bool {
        !matches!(
            self,
            Edit::Move(_)
                | Edit::Select(_)
                | Edit::SelectAll
                | Edit::Copy
                | Edit::Save
                | Edit::Reload
                | Edit::Link
        )
    }
}

impl ViewPane {
    /// Start editing at the top of what is on screen rather than at the top of the file — the
    /// caret goes where the reader already is.
    pub(crate) fn edit_here(&mut self, cols: u16, rows: u16) {
        if !self.editable() {
            return;
        }
        let row = self.scroll;
        self.start_editing(cols);
        let to = {
            let cache = self.lines_for(cols);
            (row..cache.lines.len())
                .find(|&r| !super::caret::stops(&cache.lines[r]).is_empty())
                .map(|r| r - self.scroll)
        };
        if let Some(r) = to {
            self.click_caret(r, 0, cols, rows);
        }
    }

    /// The pane's title: the file's name, and a `●` while it holds unsaved edits — the mark
    /// a document window's legend carries, for the pane that has no legend of its own.
    pub(crate) fn title(&self) -> String {
        let name = self.path.file_name().map_or_else(
            || self.path.to_string_lossy().into_owned(),
            |n| n.to_string_lossy().into_owned(),
        );
        match self.dirty {
            true => format!("{name} \u{25cf}"),
            false => name,
        }
    }

    /// Back to reading: no caret, no selection, every block rendered. The edits stay — they
    /// are unsaved, not undone.
    pub(crate) fn stop_editing(&mut self) {
        self.caret = None;
        self.caret_at = None;
        self.anchor = None;
    }

    /// Apply one editing key.
    pub(crate) fn apply_edit(&mut self, e: Edit, cols: u16, rows: u16) -> EditDone {
        if e.changes_text() {
            self.warned = false;
        }
        match e {
            Edit::Move(dir) => {
                self.clear_selection();
                self.move_caret(dir, cols, rows);
            }
            Edit::Select(dir) => {
                self.anchor_here();
                self.move_caret(dir, cols, rows);
            }
            Edit::SelectAll => self.select_all(cols, rows),
            // Bold and italic are markdown's; a code file has no spelling for them.
            Edit::Wrap(_) if !self.renders_markdown() => return EditDone::Drawn,
            Edit::Wrap(marker) if !self.wrap_selection(marker, cols, rows) => {
                return EditDone::Refused
            }
            Edit::Wrap(_) => {}
            Edit::Copy => return self.selected_text().map_or(EditDone::Drawn, EditDone::Copy),
            Edit::Cut => {
                let text = self.selected_text();
                self.delete_selection(cols, rows);
                return text.map_or(EditDone::Drawn, EditDone::Copy);
            }
            Edit::Paste => return EditDone::Paste,
            Edit::Type(text) => self.insert(&text, cols, rows),
            Edit::Backspace => self.backspace(cols, rows),
            Edit::Delete => self.delete(cols, rows),
            Edit::Undo => {
                self.undo(cols, rows);
            }
            Edit::Redo => {
                self.redo(cols, rows);
            }
            Edit::Newline if !self.renders_markdown() => self.newline_plain(cols, rows),
            Edit::Newline => self.newline(cols, rows),
            Edit::Save => return EditDone::Save,
            Edit::Reload => return EditDone::Reload,
            Edit::Tab if !self.renders_markdown() => self.tab_plain(cols, rows),
            Edit::Tab => {
                if !self.tab_cell(cols, rows) {
                    self.insert("  ", cols, rows);
                }
            }
            Edit::Link => return EditDone::Link,
        }
        EditDone::Drawn
    }
}

#[cfg(test)]
#[path = "editkeys_tests.rs"]
mod tests;

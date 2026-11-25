//! Editing a markdown document in a viewer pane, in the grid — the same editor the document
//! window is ([`crate::docwin`]), without leaving the pane.
//!
//! A pane is a reader first: Cmd+click, `/diff` and `/out` all open one to READ, and its letters
//! are commands (`e o r s v w / n [ ]`). So editing is asked for — `i` — and Esc puts it back.
//! While it is on, the pane takes the keys an editor needs ahead of the grid's chords: a letter
//! is a letter, Cmd+Z is undo rather than zoom, Cmd+S saves this file. Cmd+arrows still walk the
//! grid, and every chord the editor has no use for still reaches the app.
//!
//! The edits are the pane's until they are saved, so every way a pane can lose them — closing
//! it, re-reading it, handing it to `$EDITOR`, popping it out into a window (which reads the
//! file afresh) — asks once or refuses.
use winit::event::KeyEvent;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::app::CrewApp;
use crate::docwin::keys::{edit_for, Edit};
use crate::pane::PaneContent;
use crate::viewpane::{EditDone, ViewPane};

impl CrewApp {
    /// The focused viewer pane, when it is being edited.
    fn editing_view(&mut self) -> Option<(&mut ViewPane, u16, u16)> {
        let pane = self.panes.get_mut(self.focused)?;
        let (cols, rows) = (pane.grid.cols, pane.grid.rows);
        match &mut pane.content {
            PaneContent::View(v) if v.caret.is_some() => Some((v, cols, rows)),
            _ => None,
        }
    }

    /// A key for a viewer pane being edited. `true` when it was the editor's.
    pub(crate) fn view_edit_key(&mut self, event: &KeyEvent, mstate: ModifiersState) -> bool {
        // The Explorer holding the keys outranks an editing viewer behind it.
        !self.nav_has_keys()
            && self.view_edit_input(&event.logical_key, event.state.is_pressed(), mstate)
    }

    /// [`Self::view_edit_key`] over a key the tests can build — winit's `KeyEvent` cannot be.
    pub(crate) fn view_edit_input(
        &mut self,
        key: &Key,
        pressed: bool,
        mstate: ModifiersState,
    ) -> bool {
        // Hint mode's letters ARE its interface; the input bar has its own keys.
        if self.input.focused || !pressed || crate::hints::active() {
            return false;
        }
        let focused = self.focused;
        let Some((v, cols, rows)) = self.editing_view() else {
            return false;
        };
        if matches!(key, Key::Named(NamedKey::Escape)) {
            v.stop_editing();
            let dirty = v.dirty;
            self.set_status(match dirty {
                true => crate::chordglyph::shown_ctrl("reading — unsaved edits, Cmd+S to save"),
                false => "reading — i to edit",
            });
            self.redraw();
            return true;
        }
        let page = usize::from(rows).saturating_sub(1).max(1);
        let Some(e) = edit_for(key, pressed, mstate, page) else {
            return false;
        };
        // Cmd+arrows walk the grid and Cmd+K is the app's; the pane has no URL field — the
        // caret's own block shows the URL, so it is typed into in place.
        let cmd = mstate.super_key() || mstate.control_key();
        if matches!(e, Edit::Link) || (cmd && matches!(e, Edit::Move(_) | Edit::Select(_))) {
            return false;
        }
        match v.apply_edit(e, cols, rows) {
            EditDone::Save => self.save_view(focused),
            EditDone::Reload => {
                if !self.refuse_dirty(focused, "Cmd+R again to re-read the file and discard them") {
                    self.reread_view(focused);
                }
            }
            EditDone::Copy(text) => self.copy_text(text),
            EditDone::Paste => {
                let text = crate::clipboard::system_text();
                if let (Some(text), Some((v, cols, rows))) = (text, self.editing_view()) {
                    v.insert(&text, cols, rows);
                }
            }
            EditDone::Refused => {
                self.set_status("select inside one paragraph to make it bold or italic")
            }
            EditDone::Drawn | EditDone::Link => {}
        }
        self.redraw();
        true
    }

    /// A plain click in a viewer pane being edited puts the caret there.
    pub(crate) fn view_edit_click(&mut self) -> bool {
        let Some(i) = self.pane_at_cursor() else {
            return false;
        };
        let Some((row, col)) = self.cursor_rowcol(i) else {
            return false;
        };
        let pane = &mut self.panes[i];
        let (cols, rows) = (pane.grid.cols, pane.grid.rows);
        let PaneContent::View(v) = &mut pane.content else {
            return false;
        };
        if v.caret.is_none() || row < 0 || col < 0 {
            return false;
        }
        v.click_caret(row as usize, col as u16, cols, rows);
        self.focused = i;
        self.input.focused = false;
        true
    }

    /// Cmd+S in a pane: write the file, and say so.
    pub(crate) fn save_view(&mut self, i: usize) {
        let Some(PaneContent::View(v)) = self.panes.get_mut(i).map(|p| &mut p.content) else {
            return;
        };
        let name = v.path.display().to_string();
        v.warned = false;
        let said = match v.save() {
            Ok(()) => format!("saved {name}"),
            Err(e) => format!("could not save {name}: {e}"),
        };
        self.set_status(said);
    }

    /// Re-read a pane's file, dropping its edits — the caller has already asked.
    pub(crate) fn reread_view(&mut self, i: usize) {
        if let Some(PaneContent::View(v)) = self.panes.get_mut(i).map(|p| &mut p.content) {
            v.warned = false;
            v.stop_editing();
            v.reread();
        }
    }

    /// Whether to refuse losing pane `i`'s unsaved edits: the first time, it says so and
    /// refuses; the second time it lets them go. `again` names what doing it again will do.
    pub(crate) fn refuse_dirty(&mut self, i: usize, again: &'static str) -> bool {
        let Some(PaneContent::View(v)) = self.panes.get_mut(i).map(|p| &mut p.content) else {
            return false;
        };
        if !v.dirty || v.warned {
            return false;
        }
        v.warned = true;
        let name = v.path.file_name().map_or_else(
            || v.path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let (save, again) = (
            crate::chordglyph::shown_ctrl("Cmd+S to save"),
            crate::chordglyph::shown_ctrl(again),
        );
        self.set_status(format!("unsaved changes in {name} — {save}, {again}"));
        true
    }

    /// Whether a close of many panes at once (every pane, or all but `keep`) must wait: one of
    /// them holds unsaved edits, and a bulk close has no "again" that would mean "discard those".
    pub(crate) fn refuse_bulk_close(&mut self, keep: Option<usize>) -> bool {
        let dirty = (0..self.panes.len()).find(|&i| Some(i) != keep && self.view_is_dirty(i));
        let Some(i) = dirty else {
            return false;
        };
        let name = self.panes[i].title_text();
        self.set_status(format!(
            "unsaved changes in {name} — save it or close it first"
        ));
        true
    }

    /// Whether pane `i` holds unsaved edits — for the moves that have no "again": a pop-out
    /// and `$EDITOR` both read the file from disk, and would silently show it without them.
    pub(crate) fn view_is_dirty(&self, i: usize) -> bool {
        matches!(self.panes.get(i).map(|p| &p.content), Some(PaneContent::View(v)) if v.dirty)
    }
}

#[cfg(test)]
#[path = "viewedit_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "viewedit_code_tests.rs"]
mod code_tests;

#[cfg(test)]
#[path = "viewedit_crlf_tests.rs"]
mod crlf_tests;

#[cfg(test)]
#[path = "viewedit_open_tests.rs"]
mod open_tests;

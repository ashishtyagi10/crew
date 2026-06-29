//! Mouse text selection in terminal panes: click-drag highlights a range
//! (Alt+drag for a rectangular block), releasing copies it, and the highlight
//! is rendered by the terminal model. Built on alacritty's own `Selection`.
use crate::app::CrewApp;
use crate::pane::PaneContent;

/// An in-progress drag selection. `anchor` is the cell where the press landed;
/// `moved` flips true once the cursor reaches a different cell, which is what
/// distinguishes a selection drag from a plain click.
#[derive(Clone, Copy)]
pub(crate) struct Drag {
    pane: usize,
    anchor: (u16, u16),
    moved: bool,
    block: bool,
}

impl CrewApp {
    /// The terminal pane and viewport cell (col, row) under the cursor, or
    /// `None` when the cursor isn't over a terminal pane's content area. Mirrors
    /// `cursor_cell`'s geometry, including the one-row title-bar offset.
    pub(crate) fn cursor_term_cell(&self) -> Option<(usize, u16, u16)> {
        let i = self.pane_at_cursor()?;
        let (cw, ch, _sw, _sh, _scale) = self.frame_geometry()?;
        let rect = self
            .pane_hit_rects()
            .into_iter()
            .find(|&(idx, _)| idx == i)
            .map(|(_, r)| r)?;
        let col = ((self.cursor.0 - rect.x) / cw).floor() as i32;
        // Content sits one row below the pane's title bar.
        let row = ((self.cursor.1 - rect.y) / ch).floor() as i32 - 1;
        if col < 0 || row < 0 {
            return None;
        }
        matches!(self.panes.get(i)?.content, PaneContent::Terminal(_))
            .then_some((i, col as u16, row as u16))
    }

    /// On left-press: focus whatever is under the cursor and, when that's a
    /// terminal pane, arm a drag selection from this cell (clearing any prior
    /// selection). Returns the focused pane index, for double-click handling.
    pub(crate) fn selection_press(&mut self) -> Option<usize> {
        let focused = self.focus_at_cursor();
        if let Some((pane, col, row)) = self.cursor_term_cell() {
            if let Some(PaneContent::Terminal(t)) = self.panes.get_mut(pane).map(|p| &mut p.content)
            {
                t.pty.sel_clear();
            }
            let block = self.mods.state().alt_key();
            self.drag = Some(Drag {
                pane,
                anchor: (col, row),
                moved: false,
                block,
            });
        }
        focused
    }

    /// On cursor move while a drag is armed: extend the selection to the current
    /// cell. The selection only begins once the cursor leaves the anchor cell,
    /// so a click that never moves stays a click (and toggles zoom, etc.).
    pub(crate) fn selection_drag(&mut self) {
        let Some(drag) = self.drag else {
            return;
        };
        let Some((pane, col, row)) = self.cursor_term_cell() else {
            return;
        };
        if pane != drag.pane || (!drag.moved && (col, row) == drag.anchor) {
            return;
        }
        let first = !drag.moved;
        if let Some(PaneContent::Terminal(t)) =
            self.panes.get_mut(drag.pane).map(|p| &mut p.content)
        {
            if first {
                t.pty.sel_start(drag.anchor.0, drag.anchor.1, drag.block);
            }
            t.pty.sel_update(col, row);
        }
        if let Some(d) = self.drag.as_mut() {
            d.moved = true;
        }
        self.redraw();
    }

    /// On left-release: if a selection drag actually moved, copy the selected
    /// text to the clipboard and report it. Returns true when this release was a
    /// drag-select (so the caller skips click/double-click handling).
    pub(crate) fn selection_release(&mut self) -> bool {
        let Some(drag) = self.drag.take() else {
            return false;
        };
        if !drag.moved {
            return false;
        }
        if let Some(text) = self.pane_selection_text(drag.pane) {
            self.copy_text(text);
        }
        true
    }

    /// The selected text of pane `i`, if it's a terminal with a selection.
    pub(crate) fn pane_selection_text(&self, i: usize) -> Option<String> {
        match &self.panes.get(i)?.content {
            PaneContent::Terminal(t) => t.pty.sel_text(),
            _ => None,
        }
    }

    /// Copy `text` to the system clipboard and flash a status line.
    pub(crate) fn copy_text(&mut self, text: String) {
        let n = text.chars().count();
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(text);
            self.set_status(format!("copied {n} chars"));
        }
    }
}

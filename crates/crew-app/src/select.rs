//! Mouse text selection: click-drag highlights a range, releasing copies it.
//! Terminal panes select through alacritty's own `Selection` (Alt+drag for a
//! rectangular block); every other pane kind selects through the pane-agnostic
//! [`crate::gridsel`] path, which works off the rendered cell grid.
use std::time::{Duration, Instant};

use crate::app::CrewApp;
use crate::gridsel::CellSel;
use crate::pane::PaneContent;

/// Max gap between two left clicks on the same pane to count as a double-click.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

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
    /// The pane and viewport cell `(col, row)` under the cursor, for *any* pane
    /// kind, or `None` when the cursor isn't over a pane's content area. Mirrors
    /// `cursor_cell`'s geometry, including the one-row title-bar offset.
    pub(crate) fn cursor_any_cell(&self) -> Option<(usize, u16, u16)> {
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
        Some((i, col as u16, row as u16))
    }

    /// As [`Self::cursor_any_cell`], but only when the cursor is over a terminal
    /// pane — used by mouse-wheel forwarding into full-screen programs.
    pub(crate) fn cursor_term_cell(&self) -> Option<(usize, u16, u16)> {
        let (i, col, row) = self.cursor_any_cell()?;
        matches!(self.panes.get(i)?.content, PaneContent::Terminal(_)).then_some((i, col, row))
    }

    /// On left-press: focus whatever is under the cursor and arm a drag
    /// selection from this cell, clearing any prior selection (terminal or
    /// generic). Returns the focused pane index, for double-click handling.
    pub(crate) fn selection_press(&mut self) -> Option<usize> {
        let focused = self.focus_at_cursor();
        self.cell_sel = None;
        if let Some((pane, col, row)) = self.cursor_any_cell() {
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
        let Some((pane, col, row)) = self.cursor_any_cell() else {
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
        } else {
            // Non-terminal pane: track a generic grid selection it can render.
            self.cell_sel = Some(CellSel {
                pane: drag.pane,
                anchor: drag.anchor,
                cursor: (col, row),
            });
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

    /// Double-click bookkeeping for a press that focused pane `i`: a second
    /// plain click on the same pane within 400 ms toggles zoom, cancelling
    /// the just-armed drag so the release doesn't copy. A press that armed a
    /// fold toggle (`fold_armed`, see `chatfold`) never counts in either
    /// direction — folding a card twice must not accidentally zoom the pane.
    pub(crate) fn click_zoom(&mut self, i: usize, fold_armed: bool) {
        let now = Instant::now();
        let double = !fold_armed
            && self
                .last_click
                .is_some_and(|(t, pi)| pi == i && now.duration_since(t) < DOUBLE_CLICK);
        if double {
            self.zoomed = !self.zoomed;
            self.last_click = None;
            self.drag = None;
        } else {
            self.last_click = (!fold_armed).then_some((now, i));
        }
    }

    /// The selected text of pane `i`: from the terminal model for terminals, or
    /// from the rendered cell grid for any other pane with a generic selection.
    pub(crate) fn pane_selection_text(&self, i: usize) -> Option<String> {
        let pane = self.panes.get(i)?;
        if let PaneContent::Terminal(t) = &pane.content {
            return t.pty.sel_text();
        }
        let sel = self.cell_sel.as_ref().filter(|s| s.pane == i)?;
        let text = crate::gridsel::selection_text(&pane.cells(false), sel);
        (!text.is_empty()).then_some(text)
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

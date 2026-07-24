//! Mouse selection on the terminal grid: anchor tracking, drag updates,
//! and selected-text extraction. Split from `model.rs` (child module —
//! parent-private access preserved).
use super::*;

impl TermCore {
    /// Map a viewport cell (0-based from the top-left of the visible area) to a
    /// grid `Point`, inverting the display offset that `cells()` applies — so a
    /// selection lines up while scrolled back into history. Clamped to the grid.
    fn viewport_point(&self, col: u16, row: u16) -> Point {
        let grid = self.term.grid();
        let off = grid.display_offset() as i32;
        let last_col = grid.columns().saturating_sub(1);
        let last_row = grid.screen_lines().saturating_sub(1) as u16;
        Point::new(
            Line(row.min(last_row) as i32 - off),
            Column((col as usize).min(last_col)),
        )
    }

    /// Begin a selection at viewport cell (col, row). `block` selects a
    /// rectangular column range rather than a linear character range.
    pub(crate) fn sel_start(&mut self, col: u16, row: u16, block: bool) {
        let point = self.viewport_point(col, row);
        let ty = if block {
            SelectionType::Block
        } else {
            SelectionType::Simple
        };
        self.sel_anchor = Some((point, ty));
        self.term.selection = Some(Selection::new(ty, point, Side::Left));
    }

    /// Extend the active selection's end to viewport cell (col, row), keeping
    /// both end cells inclusive whichever way the drag runs.
    ///
    /// The sides cannot be fixed: `to_range` swaps the anchors when the drag
    /// runs backwards but KEEPS their sides, then trims the last cell if
    /// `end.side == Left` and the first if `start.side == Right`. With a
    /// hard-coded (Left, Right) pair a backward drag swapped to (Right, Left)
    /// and lost a character off EACH end — dragging right-to-left across
    /// "hello" copied "ell". Alacritty itself avoids this by deriving the side
    /// from where in the cell the pointer sits; we only have whole cells, so
    /// derive it from the direction instead: the pair must come out (Left,
    /// Right) *after* any swap.
    pub(crate) fn sel_update(&mut self, col: u16, row: u16) {
        let point = self.viewport_point(col, row);
        let Some((anchor, ty)) = self.sel_anchor else {
            return;
        };
        let (anchor_side, cursor_side) = if point < anchor {
            (Side::Right, Side::Left)
        } else {
            (Side::Left, Side::Right)
        };
        let mut sel = Selection::new(ty, anchor, anchor_side);
        sel.update(point, cursor_side);
        self.term.selection = Some(sel);
    }

    pub(crate) fn sel_clear(&mut self) {
        self.term.selection = None;
        self.sel_anchor = None;
    }

    /// The selected text, or `None` when there's no (non-empty) selection.
    pub(crate) fn sel_text(&self) -> Option<String> {
        self.term.selection_to_string().filter(|s| !s.is_empty())
    }
}

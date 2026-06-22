//! Block-cursor overlay for the rendered terminal grid. alacritty's renderable
//! content reports the cursor position separately from the cells, so we draw it
//! ourselves as an inverted block.
use alacritty_terminal::term::RenderableCursor;
use alacritty_terminal::vte::ansi::CursorShape;

use crate::model::RenderCell;

/// Light grey for the block cursor.
const CURSOR: (u8, u8, u8) = (200, 200, 200);

/// Overlay a block cursor onto `out` at the cursor position. Only drawn when the
/// view is at the live bottom (`off == 0`) and the cursor is not hidden — when
/// scrolled into history there is no live cursor to show.
pub(crate) fn apply(out: &mut Vec<RenderCell>, cursor: &RenderableCursor, off: i32) {
    if off != 0 || matches!(cursor.shape, CursorShape::Hidden) || cursor.point.line.0 < 0 {
        return;
    }
    let col = cursor.point.column.0 as u16;
    let row = cursor.point.line.0 as u16;
    if let Some(cell) = out.iter_mut().find(|c| c.col == col && c.row == row) {
        // Invert the glyph under the cursor so it reads as a block cursor.
        cell.bg = CURSOR;
        cell.fg = (0, 0, 0);
    } else {
        out.push(RenderCell {
            col,
            row,
            c: ' ',
            fg: (0, 0, 0),
            bg: CURSOR,
            bold: false,
            italic: false,
        });
    }
}

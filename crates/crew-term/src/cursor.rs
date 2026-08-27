//! Block-cursor overlay for the rendered terminal grid. alacritty's renderable
//! content reports the cursor position separately from the cells, so we draw it
//! ourselves as an inverted block.
use alacritty_terminal::term::RenderableCursor;
use alacritty_terminal::vte::ansi::CursorShape;

use crate::model::RenderCell;

/// Overlay a block cursor onto `out` at the cursor position. Only drawn when the
/// view is at the live bottom (`off == 0`) and the cursor is not hidden — when
/// scrolled into history there is no live cursor to show. The block is the
/// page's own ink in the focused pane and half-way back to the page elsewhere.
///
/// The two used to be the constants (200,200,200) and (90,90,100), chosen by
/// eye on a dark page and never measured against a light one — where the
/// FOCUSED cursor read at 1.5 against the page and the unfocused ones at 6.2,
/// so the pane you were typing in had the faintest cursor on the canvas. They
/// come from [`crew_theme::readable`] now, which measures.
pub(crate) fn apply(out: &mut Vec<RenderCell>, cursor: &RenderableCursor, off: i32, focused: bool) {
    if off != 0 || matches!(cursor.shape, CursorShape::Hidden) || cursor.point.line.0 < 0 {
        return;
    }
    let theme = crew_theme::theme();
    let bg = crew_theme::readable::cursor(theme, focused);
    // The glyph under a block cursor is still a glyph: it takes the page's own
    // colour, pushed until it reads against whatever the block ended up being.
    let fg = crew_theme::readable::on_block(theme, bg);
    let col = cursor.point.column.0 as u16;
    let row = cursor.point.line.0 as u16;
    if let Some(cell) = out.iter_mut().find(|c| c.col == col && c.row == row) {
        // Invert the glyph under the cursor so it reads as a block cursor.
        cell.bg = bg;
        cell.fg = fg;
    } else {
        out.push(RenderCell {
            col,
            row,
            c: ' ',
            fg,
            bg,
            bold: false,
            italic: false,
            ..Default::default()
        });
    }
}

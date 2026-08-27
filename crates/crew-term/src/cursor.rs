//! Cursor overlay for the rendered terminal grid. alacritty's renderable
//! content reports the cursor position and its DECSCUSR shape separately from
//! the cells, so we draw it ourselves: a filled block by inverting the cell, a
//! bar or a rule as a mark the renderer turns into quads.
use alacritty_terminal::term::RenderableCursor;
use alacritty_terminal::vte::ansi::CursorShape as TermShape;
use crew_theme::deco::{CursorMark, CursorShape};

use crate::model::RenderCell;

/// The shape this pane draws, given what the program asked for.
///
/// An unfocused pane always draws the outline, whatever shape it is otherwise
/// in: with several panes on one canvas the useful question is *which* pane
/// takes the keys, and that reads as a shape at a glance in a way that a
/// dimmer version of the same block never did.
pub(crate) fn shape_for(asked: TermShape, focused: bool) -> CursorShape {
    match (asked, focused) {
        (TermShape::Hidden, _) => CursorShape::None,
        (_, false) => CursorShape::Hollow,
        (TermShape::Block, _) => CursorShape::Block,
        (TermShape::Beam, _) => CursorShape::Beam,
        (TermShape::Underline, _) => CursorShape::Underline,
        (TermShape::HollowBlock, _) => CursorShape::Hollow,
    }
}

/// Overlay the cursor onto `out`. Only drawn when the view is at the live
/// bottom (`off == 0`) and the cursor is not hidden — when scrolled into
/// history there is no live cursor to show. The block is the page's own ink in
/// the focused pane; an unfocused outline keeps the dimmer unfocused ink, but
/// floored against the page, because an outline is a fraction of the ink a
/// filled block is and the colour was chosen for the block.
///
/// The two colours used to be the constants (200,200,200) and (90,90,100),
/// chosen by eye on a dark page and never measured against a light one — where
/// the FOCUSED cursor read at 1.5 against the page and the unfocused ones at
/// 6.2, so the pane you were typing in had the faintest cursor on the canvas.
/// They come from [`crew_theme::readable`] now, which measures.
pub(crate) fn apply(out: &mut Vec<RenderCell>, cursor: &RenderableCursor, off: i32, focused: bool) {
    let shape = shape_for(cursor.shape, focused);
    if off != 0 || shape == CursorShape::None || cursor.point.line.0 < 0 {
        return;
    }
    let theme = crew_theme::theme();
    let bg = crew_theme::readable::cursor(theme, focused);
    // The glyph under a block cursor is still a glyph: it takes the page's own
    // colour, pushed until it reads against whatever the block ended up being.
    let fg = crew_theme::readable::on_block(theme, bg);
    let rule = crate::contrast::ensure_min_contrast(bg, theme.page_bg);
    let mark = CursorMark { shape, color: rule };
    let col = cursor.point.column.0 as u16;
    let row = cursor.point.line.0 as u16;
    if let Some(cell) = out.iter_mut().find(|c| c.col == col && c.row == row) {
        cell.cursor = mark;
        // A filled block inverts the glyph under it; every other shape is a
        // rule drawn beside the glyph, which keeps its own colours.
        if shape == CursorShape::Block {
            cell.bg = bg;
            cell.fg = fg;
        }
    } else {
        out.push(RenderCell {
            col,
            row,
            c: ' ',
            fg,
            bg: match shape {
                CursorShape::Block => bg,
                _ => crew_theme::theme().page_bg,
            },
            cursor: mark,
            ..Default::default()
        });
    }
}

#[cfg(test)]
#[path = "cursor_tests.rs"]
mod cursor_tests;

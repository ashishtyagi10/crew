//! The caret on screen: a beam on the cell it stands on.
use crew_render::CellView;

use super::caret::Caret;

/// Put the beam on the caret's cell. A beam rather than a block: the character
/// under it is the document, and a block would hide the very letter you are
/// about to type beside.
///
/// The stop after a line's last character has no rendered cell of its own —
/// and that is where the caret stands while you type at the end of a line. It
/// gets a blank cell to wear the beam, or the caret vanishes the moment you
/// type. A row exactly as wide as the pane puts that stop at `cols`, one past
/// the last column: the beam is drawn there anyway, on the cell's leading
/// edge — the pane's padding, just after the last letter — rather than lost.
pub(super) fn draw(
    out: &mut Vec<CellView>,
    caret: Caret,
    top: usize,
    (cols, rows): (u16, u16),
    page_bg: (u8, u8, u8),
) {
    let Some(row) = caret
        .row
        .checked_sub(top)
        .filter(|r| *r < usize::from(rows))
    else {
        return;
    };
    let row = row as u16;
    let mark = crew_theme::deco::CursorMark {
        shape: crew_theme::deco::CursorShape::Beam,
        color: crate::palette::accent(),
    };
    if let Some(cell) = out.iter_mut().find(|c| c.row == row && c.col == caret.col) {
        cell.cursor = mark;
    } else if caret.col <= cols {
        out.push(CellView {
            col: caret.col,
            row,
            c: ' ',
            bg: page_bg,
            cursor: mark,
            ..Default::default()
        });
    }
}

#[cfg(test)]
#[path = "caretdraw_tests.rs"]
mod tests;

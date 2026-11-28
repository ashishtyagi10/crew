//! A card's scrollbar (the user, 2026-11-25: "both panel should have
//! scrollbar if needed"): while its words run past its rows, a heavy run of
//! the accent on the card's right edge, as long as the share shown and as
//! far down as the first row shown. A card that fits keeps its plain edge.
use super::cards::Rect;
use super::draw::Canvas;

/// The thumb on `b`'s right edge on `cv`, for `total` rows of which `room`
/// show from `first`, the shown rows starting `top` rows below the card's
/// top edge.
pub(crate) fn draw(cv: &mut Canvas, b: Rect, total: usize, room: usize, first: usize, top: u16) {
    let Some((at, len)) = crate::chatscroll::thumb(total, room, first) else {
        return;
    };
    if b.w < 6 || b.h < 4 {
        return;
    }
    let col = b.col + b.w - 1;
    let bottom = b.row + b.h - 1;
    let fg = crate::palette::accent();
    for i in at..at + len {
        let row = b.row + top + i as u16;
        if row <= b.row || row >= bottom {
            continue;
        }
        // The edge's own `│` gives way, so one cell is drawn there.
        cv.out.retain(|c| !(c.row == row && c.col == col));
        cv.text(row, col, col + 1, "\u{2503}", fg, false);
    }
}

#[cfg(test)]
#[path = "thumb_tests.rs"]
mod tests;

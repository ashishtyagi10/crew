//! The failed command's mark on a card's left border, as a click target.
//!
//! The mark is drawn by [`crate::cardmarks::marks`] at a row [`rows`] names,
//! and a click is tested against that same row at the same y
//! ([`crate::cardmarks::row_y`]) — so what is drawn and what answers a click
//! cannot drift apart, and a click lands on the failed mark and nothing else.
use crate::cmdspan::Span;
use crate::layout::Rect;
use crate::pane::TermPane;

/// The visible rows that begin a command the shell reported failed, with
/// the command — what the card draws in the alarm colour and what a click on
/// the border answers.
pub(crate) fn in_view(t: &TermPane, grid_rows: u16) -> Vec<(u16, &Span)> {
    let (now, off) = (t.pty.lines_seen(), t.pty.display_offset());
    t.spans.failed_in_view(now, usize::from(grid_rows), off)
}

/// [`in_view`]'s rows alone, for the drawing.
pub(crate) fn rows(t: &TermPane, grid_rows: u16) -> Vec<u16> {
    in_view(t, grid_rows).into_iter().map(|(r, _)| r).collect()
}

/// The failed row under the pointer at `cursor` px, if any: on the card's
/// left border cell, in a row of `fail_rows`, on a `cw`×`ch` px cell.
pub(crate) fn hit(
    rect: Rect,
    (cw, ch): (f32, f32),
    cursor: (f32, f32),
    fail_rows: &[u16],
) -> Option<u16> {
    if !(rect.x..rect.x + cw).contains(&cursor.0) {
        return None;
    }
    let dy = crate::panefit::slack(rect, cw, ch).1 / ch;
    fail_rows.iter().copied().find(|&r| {
        let top = rect.y + crate::cardmarks::row_y(r, dy) * ch;
        (top..top + ch).contains(&cursor.1)
    })
}

#[cfg(test)]
#[path = "failmark_tests.rs"]
mod tests;

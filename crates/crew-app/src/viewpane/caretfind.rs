//! Where a caret is found again: under a click, or after the document was
//! laid out at a new width. Split from [`super::caret`], which is about where
//! a caret can step; these are about where one lands.
use super::caret::{first, last, stops, Caret};
use crate::chatbody::CardLine;

/// The caret nearest a clicked cell: the last position at or before `col` on
/// that row, or the row's end when the click was past everything on it.
///
/// A click that lands on a row of pure furniture — a rule, a code field's
/// border — finds the nearest row that has somewhere to stand, rather than
/// doing nothing: a click always means "put it here", and the nearest here is
/// the honest answer.
pub(crate) fn at_cell(lines: &[CardLine], row: usize, col: u16) -> Option<Caret> {
    for r in (0..=row.min(lines.len().saturating_sub(1))).rev() {
        let s = stops(&lines[r]);
        if s.is_empty() {
            continue;
        }
        let at = match r == row {
            true => s
                .iter()
                .rev()
                .find(|&&(c, _)| c <= col)
                .or(s.first())
                .copied(),
            // A click below the last row with anything on it lands at its end.
            false => s.last().copied(),
        };
        let (col, _) = at?;
        return Some(Caret {
            row: r,
            col,
            want: col,
        });
    }
    first(lines)
}

/// Where the caret should be after the document was laid out again (a resize,
/// an edit): the position now holding `offset`, or the nearest one after it.
///
/// Rows carry increasing offsets, so this is a search rather than a walk — a
/// document of a hundred thousand rows must not be scanned on every keypress
/// that changes its width.
pub(crate) fn find(lines: &[CardLine], offset: u32) -> Option<Caret> {
    let key = |row: &CardLine| stops(row).first().map(|&(_, s)| s);
    let mut lo = 0usize;
    let mut hi = lines.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        match key(&lines[mid]) {
            Some(s) if s > offset => hi = mid,
            _ => lo = mid + 1,
        }
    }
    // `lo` is one past the last row that starts at or before `offset`; walk
    // back over rows the renderer filled with furniture (which have no key).
    let start = lines[..lo].iter().rposition(|l| key(l).is_some())?;
    for (row, line) in lines.iter().enumerate().skip(start) {
        if let Some(&(col, _)) = stops(line).iter().find(|&&(_, s)| s >= offset) {
            return Some(Caret {
                row,
                col,
                want: col,
            });
        }
    }
    // Past every place in the document: the end, not the start — this is a
    // caret that was at the end of a longer text.
    last(lines)
}

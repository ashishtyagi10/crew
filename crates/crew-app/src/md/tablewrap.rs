//! A table wider than its card WRAPS instead of clipping.
//!
//! `table::lines` pads every column to its widest cell, and a table whose
//! columns add up past `cols` was hard-cut at the right edge — the notes
//! column of every comparison an agent writes is the last column, and the
//! last column is exactly the one the cut took. Here the widest columns are
//! squeezed until the table fits and the cell text that no longer fits its
//! column goes onto continuation rows, separators and alignment kept, so the
//! table reads as rows of wrapped cells rather than as rows with their ends
//! torn off. Below [`MIN_COL`] a column is not a column, so a table that
//! cannot fit even at that floor falls back to the clip.
use super::super::wrap;
use crate::md::{ColAlign, MdLine, MdSpan};

/// The narrowest a squeezed column may get: a word's worth.
pub(super) const MIN_COL: usize = 6;

/// How many rows one cell may wrap onto. A cell past this is cut there —
/// the same cut the whole table took before, now at row twelve of one cell
/// rather than at column forty of every row. It is also what keeps a
/// pathological cell (the perf test's three-million-character one) from
/// being wrapped at all: the cell is clamped to this many rows of text
/// BEFORE the wrap looks at it.
const MAX_CELL_ROWS: usize = 12;

/// The columns squeezed to fit `cols`: every column wider than one level is
/// capped at it, and the level is the largest one at which the table fits.
/// `None` when even every column at [`MIN_COL`] overflows — the caller
/// keeps today's clip. Callers pass widths that already overflow.
pub(super) fn shrink(widths: &[usize], seps: usize, cols: usize) -> Option<Vec<usize>> {
    let avail = cols.checked_sub(seps)?;
    let at = |level: usize| widths.iter().map(|&w| w.min(level)).sum::<usize>();
    if at(MIN_COL) > avail {
        return None;
    }
    // Binary search the level: `at` is monotone, MIN_COL fits, the widest
    // column's own width (the natural layout) does not.
    let (mut lo, mut hi) = (MIN_COL, widths.iter().copied().max().unwrap_or(0));
    while hi > lo + 1 {
        let mid = lo + (hi - lo) / 2;
        if at(mid) <= avail {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(widths.iter().map(|&w| w.min(lo)).collect())
}

/// Word-wraps one cell to `w` DISPLAY columns (a wide glyph counts two — the
/// unit the column widths are in, so a CJK cell wraps where its column
/// ends, not twice as far). A word longer than the column is cut at it.
fn wrap_cell(spans: &[MdSpan], w: usize) -> Vec<Vec<MdSpan>> {
    let clamped = wrap::truncate_spans(spans.to_vec(), w * MAX_CELL_ROWS);
    let full: Vec<char> = clamped.iter().flat_map(|s| s.text.chars()).collect();
    if full.is_empty() {
        return vec![Vec::new()];
    }
    let bounds = wrap::span_bounds(&clamped);
    let mut out = Vec::new();
    let mut start = 0;
    while start < full.len() {
        let fit = crate::chatwidth::fit_end(&full, start, w);
        let end = match fit < full.len() {
            // Break at the last space that fits; the space itself is eaten.
            true => match full[start..fit].iter().rposition(|&c| c == ' ') {
                Some(p) if p > 0 => start + p,
                _ => fit,
            },
            false => fit,
        };
        out.push(wrap::spans_for_range(&clamped, &bounds, start, end));
        start = end;
        while full.get(start) == Some(&' ') {
            start += 1;
        }
    }
    out
}

/// One table row as wrapped lines: each cell wrapped to its column, the
/// row as tall as its tallest cell, every line laid through `row_line` so
/// the separators of a continuation row land where the header's do.
pub(super) fn row_lines(
    cells: &[Vec<MdSpan>],
    widths: &[usize],
    aligns: &[ColAlign],
    bold: bool,
    cols: usize,
) -> Vec<MdLine> {
    let empty = Vec::new();
    let wrapped: Vec<Vec<Vec<MdSpan>>> = widths
        .iter()
        .enumerate()
        .map(|(i, &w)| wrap_cell(cells.get(i).unwrap_or(&empty), w))
        .collect();
    let height = wrapped.iter().map(Vec::len).max().unwrap_or(1);
    (0..height)
        .map(|k| {
            let row: Vec<Vec<MdSpan>> = wrapped
                .iter()
                .map(|c| c.get(k).cloned().unwrap_or_default())
                .collect();
            super::row_line(&row, widths, aligns, bold, cols)
        })
        .collect()
}

#[cfg(test)]
#[path = "tablewrap_tests.rs"]
mod tests;

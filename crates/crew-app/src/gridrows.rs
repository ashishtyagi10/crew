//! Build a terminal grid's rows as char buffers in a single pass over the cells.
//! Per-row consumers (URL tinting, search highlighting) would otherwise rescan
//! every cell for each row — O(rows·cells); this is O(cells).
use crew_render::CellView;

/// `rows` char-buffers of width `cols`, each blank-filled then populated from the
/// cells on that row. Cells outside the `cols × rows` bounds are ignored.
pub(crate) fn grid_lines(cells: &[CellView], cols: u16, rows: u16) -> Vec<Vec<char>> {
    let mut lines = vec![vec![' '; cols as usize]; rows as usize];
    for c in cells {
        if (c.row as usize) < lines.len() && (c.col as usize) < cols as usize {
            lines[c.row as usize][c.col as usize] = c.c;
        }
    }
    lines
}

/// A grid row as the characters actually ON it, with the column each one
/// sits in: the blank column a full-width character's second half owns is
/// dropped rather than read as a space.
///
/// The grid is column-indexed, so `日本` is `['日', ' ', '本', ' ']` — and a
/// search for `日本` therefore matched nothing, ever. `/find` could not find
/// any text containing a full-width character. The column each kept character
/// sits in comes back with it, so a hit maps straight back onto the cells it
/// has to wash.
pub(crate) fn row_runs(line: &[char]) -> (Vec<char>, Vec<u16>) {
    let mut chars = Vec::with_capacity(line.len());
    let mut cols = Vec::with_capacity(line.len());
    let mut skip = false;
    for (i, &c) in line.iter().enumerate() {
        if std::mem::take(&mut skip) {
            continue;
        }
        skip = unicode_width::UnicodeWidthChar::width(c) == Some(2);
        chars.push(c);
        cols.push(i as u16);
    }
    (chars, cols)
}

#[cfg(test)]
#[path = "gridrows_tests.rs"]
mod tests;

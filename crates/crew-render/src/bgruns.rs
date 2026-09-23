//! Cell backgrounds as RUNS with soft corners where they meet the page.
//!
//! A highlight — a selected row, a key hint, a block cursor, a status band —
//! used to be one hard rectangle per cell. Drawn as one quad per horizontal
//! run of a colour instead, it can round its ends into a capsule, which is
//! how a highlight sits on glass rather than being stamped onto it.
//!
//! Only a corner that sits on BARE PAGE rounds: a run's top-left corner
//! rounds when the cell to its left and the cell above its first column are
//! both page. So a block several rows tall rounds its outer corners and
//! nothing between its rows (no scalloped edge), and colours that abut —
//! powerline segments, a heatmap's squares, a TUI's painted panels — keep
//! their square seams.
use crate::cellgrid::CellView;
use unicode_width::UnicodeWidthChar;

/// One horizontal run of one background colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Run {
    pub row: u16,
    pub col: u16,
    /// Width in columns (a wide character counts two).
    pub cols: u16,
    pub bg: (u8, u8, u8),
    /// Which corners round: top-left, top-right, bottom-right, bottom-left.
    pub round: [bool; 4],
}

/// The runs `cells` paint on a `cols`×`rows` grid, skipping `page` (the
/// default background, which draws nothing).
pub(crate) fn runs(cells: &[CellView], cols: usize, rows: usize, page: (u8, u8, u8)) -> Vec<Run> {
    let mut grid: Vec<Option<(u8, u8, u8)>> = vec![None; cols * rows];
    for c in cells.iter().filter(|c| c.bg != page) {
        let (row, col) = (usize::from(c.row), usize::from(c.col));
        let wide = match UnicodeWidthChar::width(c.c) {
            Some(2) => 2,
            _ => 1,
        };
        for x in col..(col + wide).min(cols) {
            if row < rows {
                grid[row * cols + x] = Some(c.bg);
            }
        }
    }
    let at = |r: isize, c: isize| -> Option<(u8, u8, u8)> {
        let inside = r >= 0 && c >= 0 && (r as usize) < rows && (c as usize) < cols;
        inside
            .then(|| grid[r as usize * cols + c as usize])
            .flatten()
    };
    let mut out = Vec::new();
    for r in 0..rows {
        let mut c = 0;
        while c < cols {
            let Some(bg) = grid[r * cols + c] else {
                c += 1;
                continue;
            };
            let start = c;
            while c < cols && grid[r * cols + c] == Some(bg) {
                c += 1;
            }
            let (ri, first, last) = (r as isize, start as isize, c as isize - 1);
            let (left, right) = (at(ri, first - 1).is_none(), at(ri, last + 1).is_none());
            out.push(Run {
                row: r as u16,
                col: start as u16,
                cols: (c - start) as u16,
                bg,
                round: [
                    left && at(ri - 1, first).is_none(),
                    right && at(ri - 1, last).is_none(),
                    right && at(ri + 1, last).is_none(),
                    left && at(ri + 1, first).is_none(),
                ],
            });
        }
    }
    out
}

/// The corner radius for a cell of `cell_w`×`cell_h` px: soft enough to read
/// as a capsule on a one-cell run, never so round that a run's end looks
/// bitten off.
pub(crate) fn radius(cell_w: f32, cell_h: f32) -> f32 {
    (cell_w * 0.5).min(cell_h * 0.3)
}

#[cfg(test)]
#[path = "bgruns_tests.rs"]
mod tests;

//! Where the view sits in the scrollback, and the search through it in one
//! pass over the grid's own rows.
//!
//! `/find` used to scroll the view one line at a time and rebuild every cell
//! of the screen (colour, contrast floor, decoration) twice per step, on the
//! window's thread, and it gave up after 5,000 steps: half of the 10,000-line
//! history, so the first error of a long agent run was "no match". It also
//! read each row alone, so a word a soft wrap split across two rows never
//! matched. This reads the characters straight out of the grid, joins the
//! rows a program ran past (`WRAPLINE`), and never moves the view.
use super::*;
use alacritty_terminal::grid::Row;
use alacritty_terminal::term::cell::Cell;

impl TermCore {
    pub(crate) fn scroll(&mut self, delta: i32) {
        self.term.scroll_display(Scroll::Delta(delta));
    }

    pub(crate) fn scroll_to_bottom(&mut self) {
        self.term.scroll_display(Scroll::Bottom);
    }

    pub(crate) fn display_offset(&self) -> usize {
        self.term.grid().display_offset()
    }

    /// Every line the viewport can reach: the scrollback history plus the live
    /// screen. The denominator of a scroll indicator — `display_offset` alone
    /// says how far back you are, never how far back there is.
    pub(crate) fn scrollable_lines(&self) -> usize {
        let g = self.term.grid();
        g.history_size() + g.screen_lines()
    }

    /// Walk the buffer's logical lines upward, starting with the row `from`
    /// lines above the screen's bottom row, and return the depth (lines above
    /// that bottom row) of the top row of the first one `hit` accepts.
    ///
    /// Rows above `from` that wrap into it are joined to it; rows below it
    /// are not read, so a repeat that starts above its last hit cannot find
    /// that hit again.
    pub(crate) fn find_back(
        &self,
        from: usize,
        hit: &mut dyn FnMut(&str) -> bool,
    ) -> Option<usize> {
        let grid = self.term.grid();
        let cols = grid.columns();
        let bottom = grid.screen_lines() as i32 - 1;
        let top = -(grid.history_size() as i32);
        let mut line = bottom - i32::try_from(from).ok()?;
        let mut text = String::new();
        while line >= top {
            let end = line;
            while line > top && wraps(&grid[Line(line - 1)], cols) {
                line -= 1;
            }
            text.clear();
            for l in line..=end {
                push_row(&grid[Line(l)], cols, &mut text);
            }
            if hit(&text) {
                return Some((bottom - line) as usize);
            }
            line -= 1;
        }
        None
    }
}

/// Whether the program ran past this row's last column onto the next row.
fn wraps(row: &Row<Cell>, cols: usize) -> bool {
    cols > 0 && row[Column(cols - 1)].flags.contains(Flags::WRAPLINE)
}

/// The row's text, one char per column of text: the blank parked behind a
/// full-width character is not a column (see `modelcells`), `\0` is a space.
fn push_row(row: &Row<Cell>, cols: usize, out: &mut String) {
    for c in 0..cols {
        let cell = &row[Column(c)];
        let spacer = Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER;
        if !cell.flags.intersects(spacer) {
            out.push(if cell.c == '\0' { ' ' } else { cell.c });
        }
    }
}

#[cfg(test)]
#[path = "modelscroll_tests.rs"]
mod modelscroll_tests;

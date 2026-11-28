//! One buffer line as text, by absolute number, without moving the view.
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::Flags;

use super::TermCore;

impl TermCore {
    /// The text of absolute line `abs` from column `from` up to `to`, and whether the
    /// program ran past its end onto the next row (a soft wrap). `None` when
    /// the line is not in the buffer (evicted, or below the screen). Wide
    /// characters' spacer cells are skipped, colours never come along.
    pub(super) fn line_text(&self, abs: u64, from: usize, to: usize) -> Option<(String, bool)> {
        let grid = self.term.grid();
        let rel = abs as i64 - self.screen_line() as i64;
        if rel < -(grid.history_size() as i64) || rel >= grid.screen_lines() as i64 {
            return None;
        }
        let row = &grid[Line(rel as i32)];
        let cols = grid.columns();
        let to = to.min(cols);
        let spacer = Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER;
        let mut text = String::new();
        for c in from..to {
            let cell = &row[Column(c)];
            if cell.flags.intersects(spacer) {
                continue;
            }
            text.push(cell.c);
            text.extend(cell.zerowidth().into_iter().flatten());
        }
        let wrapped = cols > 0 && row[Column(cols - 1)].flags.contains(Flags::WRAPLINE);
        Some((text, wrapped))
    }
}

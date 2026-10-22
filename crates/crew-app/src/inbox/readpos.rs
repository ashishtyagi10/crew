//! Where the open thread is in the list, on the mail card's top edge:
//! `‹ 3 of 25 ›` at the right, each arrow a click to the previous or the
//! next thread, as `p` and `n` are (the reader loop, round 1: "navigation
//! should be simple"). On the edge it stays put while the mail scrolls.
use crate::chatwidth::str_w;

use super::InboxPane;

impl InboxPane {
    /// The counter's words; none with no list to count.
    pub(crate) fn counter(&self) -> Option<String> {
        if self.reading.is_none() || self.rows.is_empty() {
            return None;
        }
        let more = if self.more { "+" } else { "" };
        let n = self.rows.len();
        Some(format!("\u{2039}  {} of {n}{more}  \u{203a}", self.sel + 1))
    }

    /// The pane's `(row, col)` on one of the counter's arrows: `Some(true)`
    /// for the next (older) thread, `Some(false)` the previous.
    pub(crate) fn counter_at(&self, cols: u16, rows: u16, row: u16, col: u16) -> Option<bool> {
        let s = self.counter()?;
        let b = self.cards(cols, rows).mail.filter(|b| b.row == row)?;
        // As `cards::frame` sets it: a space each side, then the corner.
        let end = (b.col + b.w).saturating_sub(3);
        let start = end.saturating_sub(str_w(&s) as u16);
        match col {
            c if (start.saturating_sub(1)..start + 2).contains(&c) => Some(false),
            c if (end.saturating_sub(2)..end + 1).contains(&c) => Some(true),
            _ => None,
        }
    }
}

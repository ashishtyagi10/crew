//! Where the open thread is in the list, on the subject's row: `‹ 3 of 25 ›`
//! at the right, each arrow a click to the previous or the next thread, as
//! `p` and `n` are (the reader loop, round 1: "navigation should be simple").
use crate::chatwidth::str_w;

use super::InboxPane;

/// The columns the subject leaves the counter on its first row.
pub(crate) const ROOM: usize = 18;

impl InboxPane {
    /// The counter's words and its first column, in a reader `cols` wide;
    /// none once the thread has scrolled, or with no list to count.
    pub(crate) fn counter(&self, cols: u16) -> Option<(String, u16)> {
        let r = self.reading.as_ref()?;
        if r.scroll > 0 || self.rows.is_empty() {
            return None;
        }
        let more = if self.more { "+" } else { "" };
        let n = self.rows.len();
        let s = format!("\u{2039}  {} of {n}{more}  \u{203a}", self.sel + 1);
        // The reader's words end two columns in from its edge.
        let end = cols.saturating_sub(2);
        Some((s.clone(), end.saturating_sub(str_w(&s) as u16)))
    }

    /// `(row, col)` of a reader `cols` wide on one of the counter's arrows:
    /// `Some(true)` for the next (older) thread, `Some(false)` the previous.
    pub(crate) fn counter_at(&self, cols: u16, row: u16, col: u16) -> Option<bool> {
        let (s, start) = self.counter(cols).filter(|_| row == 1)?;
        let end = start + str_w(&s) as u16;
        match col {
            c if (start.saturating_sub(1)..start + 2).contains(&c) => Some(false),
            c if (end - 2..end + 1).contains(&c) => Some(true),
            _ => None,
        }
    }
}

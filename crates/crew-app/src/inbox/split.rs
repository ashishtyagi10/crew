//! The list and the open thread side by side on a wide pane (goal
//! 2026-10-20-inbox-list-look, phase 5; the user: "side-by-side on wide
//! panes"). At [`WIDE`] columns or more an open thread reads on the right and
//! the list stays on the left, two fifths of the pane between 48 and 64
//! columns, each in its own card (`cards`). Narrower, the thread takes the
//! pane as it always has.
//!
//! Everything that measures the reader — its drawing, its pictures, its
//! links by click and by hint, its scrolling — asks [`InboxPane::reader_box`]
//! where it is, so none of it can think the thread is the width of the pane.
use super::InboxPane;

/// The pane width the two halves need.
pub(crate) const WIDE: u16 = 140;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Split {
    /// Where the list's card ends (the pane's columns before it).
    pub list: u16,
    /// Where the mail's card begins, a column of air after the list's.
    pub at: u16,
}

impl InboxPane {
    /// The two halves, when a thread is open in a pane `cols` wide enough.
    pub(crate) fn split(&self, cols: u16) -> Option<Split> {
        (self.reading.is_some() && cols >= WIDE).then(|| {
            let list = (cols * 2 / 5).clamp(48, 64);
            Split { list, at: list + 1 }
        })
    }

    /// The mail card's first column and width, its edges included.
    pub(crate) fn mail_cols(&self, cols: u16) -> (u16, u16) {
        match self.split(cols) {
            Some(s) => (s.at, cols.saturating_sub(1 + s.at)),
            None => (1, cols.saturating_sub(2)),
        }
    }

    /// Where the open thread's words are drawn in a pane `cols` wide: the
    /// first column inside its card, and the width inside its edges.
    pub(crate) fn reader_box(&self, cols: u16) -> (u16, u16) {
        let (col, w) = self.mail_cols(cols);
        (col + 1, w.saturating_sub(2))
    }
}

#[cfg(test)]
#[path = "split_tests.rs"]
mod tests;

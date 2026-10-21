//! The list and the open thread side by side on a wide pane (goal
//! 2026-10-20-inbox-list-look, phase 5; the user: "side-by-side on wide
//! panes"). At [`WIDE`] columns or more an open thread reads on the right and
//! the list stays on the left, two fifths of the pane between 48 and 64
//! columns, a hairline in the air between them. Narrower, the thread takes
//! the pane as it always has.
//!
//! Everything that measures the reader — its drawing, its pictures, its
//! links by click and by hint, its scrolling — asks [`InboxPane::reader_box`]
//! where it is, so none of it can think the thread is the width of the pane.
use crew_render::Paint;

use super::InboxPane;

/// The pane width the two halves need.
pub(crate) const WIDE: u16 = 140;
/// The air between the halves, the hairline in its middle.
const GUTTER: u16 = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Split {
    /// The list's width.
    pub list: u16,
    /// The reader's first column.
    pub at: u16,
}

impl InboxPane {
    /// The two halves, when a thread is open in a pane `cols` wide enough.
    pub(crate) fn split(&self, cols: u16) -> Option<Split> {
        (self.reading.is_some() && cols >= WIDE).then(|| {
            let list = (cols * 2 / 5).clamp(48, 64);
            Split {
                list,
                at: list + GUTTER,
            }
        })
    }

    /// Where the open thread is drawn in a pane `cols` wide: its first
    /// column, and its width.
    pub(crate) fn reader_box(&self, cols: u16) -> (u16, u16) {
        match self.split(cols) {
            Some(s) => (s.at, cols - s.at),
            None => (0, cols),
        }
    }

    /// The hairline between the halves, `rows` tall: a quiet edge, not a
    /// rule, as the list has none.
    pub(crate) fn split_paint(&self, cols: u16, rows: u16) -> Option<Paint> {
        let s = self.split(cols)?;
        let t = crew_theme::theme();
        let x = f32::from(s.list) + f32::from(GUTTER) / 2.0;
        Some(Paint {
            alpha: 0.5,
            ..Paint::solid(
                x,
                1.0,
                0.08,
                f32::from(rows.saturating_sub(2)),
                t.text_muted,
            )
        })
    }
}

#[cfg(test)]
#[path = "split_tests.rs"]
mod tests;

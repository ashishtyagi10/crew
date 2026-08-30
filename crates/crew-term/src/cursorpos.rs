//! Where the live cursor is, as a plain viewport cell.
//!
//! The cursor already reaches the renderer as a mark on a cell
//! ([`crate::cursor::apply`]), which is everything needed to *draw* it — and
//! nothing at all for the question the app asks between frames: did it move,
//! and from where. That answer wants the position on its own, cheaply, without
//! rebuilding the grid — `crate::modelcells` walks every visible cell, and a
//! caret animation reading it once per pane per frame would double the most
//! expensive pass in the terminal.
//!
//! `None` means "there is nothing on screen to follow": the program hid the
//! cursor, or the view is scrolled into history, where the live cursor is not
//! what is being looked at. Both are exactly the conditions
//! [`crate::cursor::apply`] refuses to draw under, so a follower can never
//! trail a caret that is not on the page.
use super::*;

impl TermCore {
    /// The live cursor's viewport cell, or `None` when it is hidden or the
    /// view is scrolled back.
    pub(crate) fn cursor_cell(&self) -> Option<(u16, u16)> {
        if !self.term.mode().contains(TermMode::SHOW_CURSOR) || self.display_offset() != 0 {
            return None;
        }
        let p = self.term.grid().cursor.point;
        (p.line.0 >= 0).then_some((p.column.0 as u16, p.line.0 as u16))
    }
}

//! Where the sidebar's LOG window sits, and how a wheel moves it.
//!
//! The LOG is five rows onto a hundred buffered entries; the other ninety-five
//! were reachable only through `/log`, which opens a pane for something the
//! sidebar was already showing part of. Split from [`crate::navlog`], which
//! draws the window this decides the position of.
use crate::app::CrewApp;
use crate::navlog::max_back;

impl CrewApp {
    /// A wheel over the sidebar's LOG scrolls it instead of a pane. Returns
    /// `true` when the wheel was claimed.
    pub(crate) fn scroll_log_at_cursor(&mut self, lines: i32) -> bool {
        if self.log.is_empty() {
            return false;
        }
        let Some((sb, ch, l)) = self.nav_hit_geometry() else {
            return false;
        };
        if l.log_lines == 0 || !crate::chrome::point_in(sb, self.cursor.0, self.cursor.1) {
            return false;
        }
        // Measured from the card's OUTER top edge: +1 for the border row, +1
        // again to skip the section rule.
        let row = ((self.cursor.1 - sb.y) / ch).floor() as u16;
        let top = l.log_top + 2;
        if row < top || row >= top + l.log_lines as u16 {
            return false;
        }
        let max = max_back(self.log.len(), l.log_lines);
        let back = self.log_back as i64 + i64::from(lines);
        self.log_back = back.clamp(0, max as i64) as usize;
        true
    }

    /// A new entry arrived. Scrolled back, hold the same lines in view by
    /// stepping the offset with the buffer; at the live edge, follow. Clamped,
    /// so eviction from the front of a full buffer cannot strand the window
    /// past the oldest entry.
    pub(crate) fn log_pin_on_append(&mut self) {
        if self.log_back == 0 {
            return;
        }
        // Clamped against the widest window the nav can give the LOG: the real
        // window is frame geometry, which an append does not have.
        let max = max_back(self.log.len(), crate::navlayout::LOG_MAX);
        self.log_back = (self.log_back + 1).min(max);
    }
}

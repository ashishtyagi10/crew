//! Where the sidebar's LOG window sits, and how a wheel moves it.
//!
//! The LOG is five rows onto a hundred buffered entries; the other ninety-five
//! were reachable only through `/log`, which opens a pane for something the
//! sidebar was already showing part of. Split from [`crate::navlog`], which
//! draws the window this decides the position of.
use crate::app::CrewApp;
use crate::navlog::{max_back, LOG_LINES};

impl CrewApp {
    /// The sidebar rows the LOG occupies, as `(top, bottom)` cell rows
    /// measured from the stats card's OUTER top edge — the same frame
    /// `hit::pane_at_sidebar` measures its rows in, so the two hit paths
    /// cannot disagree about where the log ends and the pane list begins.
    fn log_rows(&self) -> (u16, u16) {
        // +1 for the card's border row, +1 again to skip the section rule.
        let top = self.sidebar.log_top() + 2;
        (top, top + LOG_LINES as u16)
    }

    /// A wheel over the sidebar's LOG scrolls it instead of a pane. Returns
    /// `true` when the wheel was claimed.
    pub(crate) fn scroll_log_at_cursor(&mut self, lines: i32) -> bool {
        if !self.config.show_nav || self.log.is_empty() {
            return false;
        }
        let Some((_cw, ch, _sw, sh, scale)) = self.frame_geometry() else {
            return false;
        };
        let sb = crate::chrome::stats_card_rect(
            sh,
            self.nav_px(scale),
            crate::app::gap(),
            ch,
            self.update.as_ref().is_some_and(|u| !u.silent),
        );
        if !crate::chrome::point_in(sb, self.cursor.0, self.cursor.1) {
            return false;
        }
        let row = ((self.cursor.1 - sb.y) / ch).floor() as u16;
        let (top, bottom) = self.log_rows();
        if row < top || row >= bottom {
            return false;
        }
        let max = max_back(self.log.len(), LOG_LINES);
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
        self.log_back = (self.log_back + 1).min(max_back(self.log.len(), LOG_LINES));
    }
}

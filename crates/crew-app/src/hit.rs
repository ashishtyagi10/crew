//! Cursor hit-testing: which docked surface or pane sits under the pointer.
use crate::app::{CrewApp, GAP};
use crate::chrome;
use crate::session::pane_at;

impl CrewApp {
    /// Focus the surface under the cursor: the input bar, or a grid pane.
    /// Returns the pane index when a pane was focused (for double-click handling).
    pub(crate) fn focus_at_cursor(&mut self) -> Option<usize> {
        if self.cursor_in_input() {
            self.input.focused = true;
            return None;
        }
        if let Some(i) = self.pane_at_sidebar().or_else(|| self.pane_at_cursor()) {
            self.focused = i;
            self.input.focused = false;
            return Some(i);
        }
        None
    }

    /// Which pane a click on the sidebar's PANES list targets, if any.
    pub(crate) fn pane_at_sidebar(&self) -> Option<usize> {
        if !self.config.show_nav {
            return None;
        }
        let (_cw, ch, _sw, sh, scale) = self.frame_geometry()?;
        let sb = chrome::sidebar_rect(sh, self.nav_px(scale), GAP);
        if !chrome::point_in(sb, self.cursor.0, self.cursor.1) {
            return None;
        }
        let rel_row = ((self.cursor.1 - sb.y) / ch).floor() as u16;
        let first = self.sidebar.panes_top() + 1; // skip the PANES header row
        let idx = rel_row.checked_sub(first)? as usize;
        (idx < self.panes.len()).then_some(idx)
    }

    /// Whether the cursor is over the docked input bar.
    pub(crate) fn cursor_in_input(&self) -> bool {
        let Some((_cw, ch, sw, sh, scale)) = self.frame_geometry() else {
            return false;
        };
        let ih = chrome::input_h(ch);
        let content =
            chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), GAP, ih);
        let ib = chrome::inputbar_rect(content, sh, ih, GAP);
        chrome::point_in(ib, self.cursor.0, self.cursor.1)
    }

    /// Which grid pane (if any) sits under the cursor — only inside the content
    /// area, so clicks on the sidebar or input bar do not steal focus.
    pub(crate) fn pane_at_cursor(&self) -> Option<usize> {
        let (_cw, ch, sw, sh, scale) = self.frame_geometry()?;
        let ih = chrome::input_h(ch);
        let c = chrome::content_rect(sw, sh, self.config.show_nav, self.nav_px(scale), GAP, ih);
        if !chrome::point_in(c, self.cursor.0, self.cursor.1) {
            return None;
        }
        pane_at(&self.grid_rects(), self.cursor.0, self.cursor.1)
    }
}

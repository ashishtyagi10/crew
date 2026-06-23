//! Scrollback routing for panes (mouse wheel and Shift+PageUp/Down).
use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent};

/// Scroll one pane's content by `lines` (positive = up/older).
fn scroll_pane(pane: &mut Pane, lines: i32) {
    match &mut pane.content {
        PaneContent::Terminal(t) => t.pty.scroll(lines),
        PaneContent::Chat(c) => c.scroll(lines, pane.grid.cols, pane.grid.rows),
        PaneContent::Settings(_) | PaneContent::Far(_) => {}
    }
}

impl CrewApp {
    /// Route a mouse-wheel scroll to the pane under the cursor.
    pub(crate) fn scroll_at_cursor(&mut self, lines: i32) {
        if lines == 0 {
            return;
        }
        if let Some(i) = self.pane_at_cursor() {
            if let Some(pane) = self.panes.get_mut(i) {
                scroll_pane(pane, lines);
                self.redraw();
            }
        }
    }

    /// Scroll the focused pane by one page (Shift+PageUp/PageDown).
    pub(crate) fn scroll_focused_page(&mut self, up: bool) {
        if let Some(pane) = self.panes.get_mut(self.focused) {
            let page = pane.grid.rows.saturating_sub(1).max(1) as i32;
            scroll_pane(pane, if up { page } else { -page });
            self.redraw();
        }
    }

    /// Jump the focused pane to the top / bottom of its scrollback (Shift+Home/End).
    pub(crate) fn scroll_focused_end(&mut self, to_top: bool) {
        if let Some(pane) = self.panes.get_mut(self.focused) {
            // The grid clamps a huge delta to the available history range.
            scroll_pane(pane, if to_top { i32::MAX / 2 } else { i32::MIN / 2 });
            self.redraw();
        }
    }
}

//! Pane-management slash commands beyond the per-pane chords. `/only` closes
//! every pane except the focused one — a quick "focus mode", like tmux's
//! kill-other-panes / zellij's pane fullscreen-by-closing. `/clearall` clears
//! every terminal pane's scrollback at once.
use crate::app::CrewApp;
use crate::pane::PaneContent;
use crew_term::TermModel;

impl CrewApp {
    /// Clear the scrollback (CSI 3 J) of every terminal pane and snap each back
    /// to its live bottom — `/clear` applied across all panes.
    pub(crate) fn clear_all_scrollback(&mut self) {
        let mut n = 0;
        for pane in &mut self.panes {
            if let PaneContent::Terminal(t) = &mut pane.content {
                t.pty.feed(b"\x1b[3J");
                t.pty.scroll_to_bottom();
                n += 1;
            }
        }
        if n == 0 {
            self.set_status("no terminals to clear");
        } else {
            let plural = if n == 1 { "" } else { "s" };
            self.set_status(format!("cleared {n} pane{plural}"));
        }
        self.redraw();
    }

    /// Close all panes except the focused one. A no-op (with a hint) when there
    /// is one pane or none.
    pub(crate) fn close_other_panes(&mut self) {
        if self.panes.len() <= 1 {
            self.set_status("only one pane");
            return;
        }
        let keep = self.focused.min(self.panes.len() - 1);
        self.panes.swap(0, keep);
        self.panes.truncate(1); // drops the rest (closing their PTYs)
        self.focused = 0;
        self.zoomed = false;
        self.input.focused = false;
        self.set_status("closed other panes");
        self.redraw();
    }

    /// Close every pane, returning to the welcome screen. A no-op (with a hint)
    /// when no panes are open. Resets focus to the input bar like the last
    /// `close_pane` would.
    pub(crate) fn close_all_panes(&mut self) {
        if self.panes.is_empty() {
            self.set_status("no panes open");
            return;
        }
        let n = self.panes.len();
        self.panes.clear();
        self.focused = 0;
        self.zoomed = false;
        self.input.focused = true;
        self.broadcast = false;
        self.input.broadcast = false;
        let plural = if n == 1 { "" } else { "s" };
        self.set_status(format!("closed {n} pane{plural}"));
        self.redraw();
    }
}

#[cfg(test)]
mod tests {
    use crate::app::CrewApp;
    use crate::farpane::FarPane;
    use crate::layout::Rect;
    use crate::pane::{Pane, PaneContent};
    use crew_term::GridSize;

    fn far_pane(name: &str) -> Pane {
        Pane {
            content: PaneContent::Far(FarPane::new(std::env::temp_dir())),
            grid: GridSize { cols: 80, rows: 24 },
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            label: None,
            name: Some(name.to_string()),
            activity: false,
            bell: false,
        }
    }

    #[test]
    fn close_others_keeps_the_focused_pane() {
        let mut app = CrewApp::default();
        for n in ["a", "b", "c"] {
            app.panes.push(far_pane(n));
        }
        app.focused = 1; // the "b" pane
        app.zoomed = true;
        app.close_other_panes();
        assert_eq!(app.panes.len(), 1);
        assert_eq!(app.focused, 0);
        assert_eq!(app.panes[0].name.as_deref(), Some("b"));
        assert!(!app.zoomed);
    }

    #[test]
    fn close_others_is_a_noop_with_one_pane() {
        let mut app = CrewApp::default();
        app.panes.push(far_pane("solo"));
        app.close_other_panes();
        assert_eq!(app.panes.len(), 1);
        assert_eq!(app.panes[0].name.as_deref(), Some("solo"));
    }

    #[test]
    fn clear_all_with_no_terminals_is_a_safe_noop() {
        let mut app = CrewApp::default();
        app.panes.push(far_pane("a"));
        // Far panes aren't terminals: nothing to clear, panes left intact.
        app.clear_all_scrollback();
        assert_eq!(app.panes.len(), 1);
        assert!(app.active_status().is_some());
    }

    #[test]
    fn close_all_clears_panes_and_refocuses_input() {
        let mut app = CrewApp::default();
        app.panes.push(far_pane("a"));
        app.panes.push(far_pane("b"));
        app.focused = 1;
        app.broadcast = true;
        app.input.broadcast = true;
        app.close_all_panes();
        assert!(app.panes.is_empty());
        assert_eq!(app.focused, 0);
        assert!(app.input.focused);
        assert!(!app.broadcast && !app.input.broadcast);
    }
}

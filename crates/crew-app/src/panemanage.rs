//! Pane-management slash commands beyond the per-pane chords. `/only` closes
//! every pane except the focused one — a quick "focus mode", like tmux's
//! kill-other-panes / zellij's pane fullscreen-by-closing.
use crate::app::CrewApp;

impl CrewApp {
    /// Minimize the pane at `idx` into the left-nav PANES list (the `[-]` button
    /// on its border): it leaves the grid but keeps running; focusing it again
    /// (click its nav row, Cmd+N) restores it. Shows the nav when hidden — the
    /// pane minimizes *into* it.
    pub(crate) fn minimize_pane(&mut self, idx: usize) {
        if idx >= self.panes.len() {
            return;
        }
        self.panes[idx].hidden = true;
        self.zoomed = false;
        if !self.config.show_nav {
            self.config.show_nav = true;
            self.config.save();
        }
        if idx == self.focused {
            // Focus the nearest visible pane; with none left, the input bar.
            match self.nearest_visible(idx) {
                Some(i) => self.focused = i,
                None => self.input.focused = true,
            }
        }
        self.set_status("minimized to nav — click its PANES row to restore");
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

    /// Close every pane, returning to the welcome screen and input bar. A no-op
    /// (with a hint) when there are no panes.
    pub(crate) fn close_all_panes(&mut self) {
        if self.panes.is_empty() {
            self.set_status("no panes to close");
            return;
        }
        let n = self.panes.len();
        // Reuse close_pane so the grid LRU and empty-state modes stay consistent.
        while !self.panes.is_empty() {
            self.close_pane(self.panes.len() - 1);
        }
        self.set_status(format!("closed {n} panes"));
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
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
        }
    }

    #[test]
    fn minimize_focused_pane_moves_focus_to_nearest_visible() {
        let mut app = CrewApp::default();
        for n in ["a", "b", "c"] {
            app.panes.push(far_pane(n));
        }
        app.focused = 1;
        app.input.focused = false;
        app.zoomed = true;
        app.minimize_pane(1);
        assert!(app.panes[1].hidden);
        assert_eq!(app.focused, 0, "nearest visible pane takes focus");
        assert!(!app.input.focused);
        assert!(!app.zoomed, "minimize leaves zoom");
    }

    #[test]
    fn minimize_unfocused_pane_keeps_focus() {
        let mut app = CrewApp::default();
        for n in ["a", "b"] {
            app.panes.push(far_pane(n));
        }
        app.focused = 0;
        app.input.focused = false;
        app.minimize_pane(1);
        assert!(app.panes[1].hidden);
        assert_eq!(app.focused, 0);
    }

    #[test]
    fn minimize_last_visible_pane_focuses_input_bar() {
        let mut app = CrewApp::default();
        app.panes.push(far_pane("solo"));
        app.focused = 0;
        app.input.focused = false;
        app.minimize_pane(0);
        assert!(app.panes[0].hidden);
        assert!(app.input.focused, "no visible pane left → input bar");
    }

    #[test]
    fn minimize_shows_the_nav_when_hidden() {
        // The pane minimizes *into* the nav, so the nav must become visible.
        let mut app = CrewApp::default();
        app.panes.push(far_pane("a"));
        app.config.show_nav = false;
        app.minimize_pane(0);
        assert!(app.config.show_nav);
    }

    #[test]
    fn minimize_out_of_range_is_a_noop() {
        let mut app = CrewApp::default();
        app.minimize_pane(3);
        assert!(app.panes.is_empty());
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
}

//! Pane-management slash commands beyond the per-pane chords. `/only` closes
//! every pane except the focused one — a quick "focus mode", like tmux's
//! kill-other-panes / zellij's pane fullscreen-by-closing.
use crate::app::CrewApp;

impl CrewApp {
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
}

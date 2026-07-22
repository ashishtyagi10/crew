//! App-side execution of the `FarAction`s a Far pane produces: from a live
//! key press (`farpane::keys::reduce`, routed via `keys.rs`) or from a
//! background remote op landing this tick (`FarPane::poll_ops`, routed via
//! `poll.rs`) — e.g. a finished `rclone` download opening its temp file.
//! Both paths share this one match so the behaviour (close the pane, open
//! the help overlay, hand a file to the OS default app, flash a status) never
//! drifts between the two call sites.
use crate::app::CrewApp;
use crate::farpane::FarAction;

impl CrewApp {
    /// Execute a `FarAction` from the Far pane at index `focused`.
    pub(crate) fn apply_far_action(&mut self, action: FarAction, focused: usize) {
        match action {
            FarAction::Close => {
                self.close_pane(focused);
            }
            FarAction::Help => self.help_open = true,
            FarAction::Open(path) => {
                let _ = open::that(path);
            }
            FarAction::Status(msg) => self.set_status(&msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::farpane::FarPane;
    use crate::pane::{Pane, PaneContent};

    fn far_pane_app() -> CrewApp {
        let mut app = CrewApp::default();
        app.panes.push(Pane {
            content: PaneContent::Far(FarPane::new(std::env::temp_dir())),
            grid: crew_term::GridSize { cols: 80, rows: 24 },
            rect: crate::layout::Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
        });
        app
    }

    #[test]
    fn close_closes_the_pane_at_focused() {
        let mut app = far_pane_app();
        assert_eq!(app.panes.len(), 1);
        app.apply_far_action(FarAction::Close, 0);
        assert_eq!(app.panes.len(), 0);
    }

    #[test]
    fn help_opens_the_help_overlay() {
        let mut app = far_pane_app();
        assert!(!app.help_open);
        app.apply_far_action(FarAction::Help, 0);
        assert!(app.help_open);
    }

    #[test]
    fn status_flashes_the_message() {
        let mut app = far_pane_app();
        app.apply_far_action(FarAction::Status("hi".into()), 0);
        assert_eq!(app.status.as_ref().map(|(msg, _)| msg.as_str()), Some("hi"));
    }

    #[test]
    fn open_does_not_panic_on_a_missing_path() {
        let mut app = far_pane_app();
        // `open::that` on a bogus path fails silently (the return is
        // discarded) — this just proves the variant is wired and doesn't
        // crash the app.
        app.apply_far_action(
            FarAction::Open(std::env::temp_dir().join("does-not-exist")),
            0,
        );
    }
}

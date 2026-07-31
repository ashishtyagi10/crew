//! App-side execution of the `FarAction`s a Far pane produces: from a live
//! key press (`farpane::keys::reduce`, routed via `keys.rs`) or from a
//! background remote op landing this tick (`FarPane::poll_ops`, routed via
//! `poll.rs`) — e.g. a finished `rclone` download opening its temp file.
//! Both paths share this one match so the behaviour (close the pane, open
//! the help overlay, show a file in the viewer or `$EDITOR`, hand a
//! downloaded remote temp file to the OS default app, flash a status) never
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
            FarAction::View(path) => self.open_view(&path.to_string_lossy()),
            FarAction::Edit(path) => self.edit_in_pane(&path.to_string_lossy()),
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
            born_ms: crate::anim::now_ms(),
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

    #[test]
    fn view_opens_the_viewer_rather_than_handing_the_file_to_the_os() {
        // Fix 6: the old assertion only checked that SOME View pane exists,
        // which passes even if `FarAction::View` opened a completely
        // unrelated path — e.g. wired to `focused`'s own directory instead
        // of the action's own `path`. Assert the opened pane's path is the
        // one the action carried.
        let f = std::env::temp_dir().join("far-view-test.txt");
        std::fs::write(&f, "x\n").unwrap();
        let mut app = far_pane_app();
        app.apply_far_action(FarAction::View(f.clone()), 0);
        let view_path = app.panes.iter().find_map(|p| match &p.content {
            PaneContent::View(v) => Some(v.path.clone()),
            _ => None,
        });
        assert_eq!(
            view_path,
            Some(f),
            "F3 must open the viewer on the action's own path"
        );
    }

    #[test]
    fn edit_spawns_a_terminal_pane_not_an_os_app() {
        let f = std::env::temp_dir().join("far-edit-test.txt");
        std::fs::write(&f, "x\n").unwrap();
        let mut app = far_pane_app();
        let before = app.panes.len();
        app.apply_far_action(FarAction::Edit(f), 0);
        assert_eq!(app.panes.len(), before + 1, "F4 opens $EDITOR in a pane");
        // Not just "a pane appeared": if `Edit` were ever mis-wired to the
        // viewer (`open_view`), pane count would still go up — a Terminal
        // pane specifically is what proves `$EDITOR` (not the viewer) ran.
        assert!(
            app.panes
                .iter()
                .any(|p| matches!(p.content, PaneContent::Terminal(_))),
            "F4 opens a terminal pane, not the viewer"
        );
    }
}

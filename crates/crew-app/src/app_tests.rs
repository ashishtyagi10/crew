use super::{slash_command, CrewApp};

fn tests_far_pane(name: &str) -> crate::pane::Pane {
    use crate::pane::{Pane, PaneContent};
    use crew_term::GridSize;
    Pane {
        content: PaneContent::Far(crate::farpane::FarPane::new(std::env::temp_dir())),
        grid: GridSize { cols: 80, rows: 24 },
        rect: crate::layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: Some(name.into()),
        name: None,
        activity: false,
        bell: false,
    }
}

#[test]
fn slash_command_parses() {
    assert_eq!(slash_command("/settings"), Some("settings"));
    assert_eq!(slash_command("/ settings "), Some("settings"));
    assert_eq!(slash_command("ls -la"), None);
    assert_eq!(slash_command("/"), Some(""));
}

#[test]
fn close_pane_resets_modes_when_empty() {
    let mut app = CrewApp {
        zoomed: true,
        broadcast: true,
        ..Default::default()
    };
    app.input.broadcast = true;
    assert!(!app.close_pane(0));
    assert!(!app.zoomed && !app.broadcast && !app.input.broadcast);
    assert!(app.input.focused);
}

#[test]
fn far_slash_command_spawns_dual_pane() {
    use crate::pane::PaneContent;
    let mut app = CrewApp::default();
    assert!(app.panes.is_empty());
    // `/far` is a non-exit command that opens a Far file-manager pane in the grid.
    assert!(!app.submit_input("/far".to_string()));
    assert_eq!(app.panes.len(), 1);
    assert!(matches!(app.panes[0].content, PaneContent::Far(_)));
    assert_eq!(app.panes[0].title_text(), "far");
}

#[test]
fn zoom_chord_toggles() {
    let mut app = CrewApp::default();
    assert!(!app.zoomed);
    app.handle_super_chord("z");
    assert!(app.zoomed);
    app.handle_super_chord("z");
    assert!(!app.zoomed);
}

#[test]
fn cd_in_input_changes_cwd_and_legend() {
    let base = std::env::temp_dir().canonicalize().unwrap();
    let mut app = CrewApp {
        cwd: base.clone(),
        ..Default::default()
    };
    // a `cd` to an existing dir is intercepted (not forwarded) and updates state.
    assert!(!app.submit_input("cd .".to_string()));
    assert_eq!(app.cwd, base);
    assert_eq!(app.input.cwd, base);
    // a non-`cd` line is not treated as a directory change.
    assert!(!app.try_change_dir("ls -la"));
}

#[test]
fn submit_without_a_shell_hints() {
    let mut app = CrewApp::default();
    // a plain command with no terminal open is not silently dropped.
    assert!(!app.submit_input("ls".to_string()));
    assert!(app.active_status().is_some());
}

#[test]
fn cd_dash_toggles_previous_directory() {
    let base = std::env::temp_dir();
    let a = base.join("crew_cd_dash_a");
    let b = base.join("crew_cd_dash_b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let (a, b) = (a.canonicalize().unwrap(), b.canonicalize().unwrap());

    let mut app = CrewApp {
        cwd: a.clone(),
        ..Default::default()
    };
    // move to b, then `cd -` returns to a, then toggles forward to b again.
    assert!(!app.submit_input(format!("cd {}", b.to_str().unwrap())));
    assert_eq!(app.cwd, b);
    assert!(!app.submit_input("cd -".to_string()));
    assert_eq!(app.cwd, a);
    assert!(!app.submit_input("cd -".to_string()));
    assert_eq!(app.cwd, b);
}

#[test]
fn reconcile_grid_tracks_panes_and_focus() {
    let mut app = CrewApp::default();
    // Simulate two spawned panes by pushing Far panes (no PTY needed).
    app.panes.push(tests_far_pane("a"));
    app.panes.push(tests_far_pane("b"));
    app.focused = 1;
    app.reconcile_grid();
    // Both panes tracked; focused (1) is most-recently-active.
    assert_eq!(app.grid.len(), 2);
    assert_eq!(app.grid.full()[0], 1);

    // Close pane 0; reconcile must not resurrect a stale index.
    app.close_pane(0);
    app.reconcile_grid();
    assert_eq!(app.grid.len(), 1);
    assert_eq!(app.grid.full(), &[0]);
}

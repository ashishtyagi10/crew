use super::{bang_command, slash_command, submit_bytes, CrewApp};

#[test]
fn submit_sends_carriage_return_not_soft_newline() {
    // A submitted input line must end in CR (0x0d) — the same byte a real Enter
    // sends — so agent CLIs (Claude/codex) submit it. Ending in LF (0x0a) is the
    // Shift+Enter "soft return", which leaves the text sitting (highlighted) in
    // the agent's input box instead of submitting it.
    assert_eq!(submit_bytes("hello"), b"hello\r");
    assert_eq!(*submit_bytes("hi").last().unwrap(), b'\r');
    assert!(!submit_bytes("hi").contains(&b'\n'));
}

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
        dir: None,
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
fn bang_command_parses() {
    assert_eq!(bang_command("!ls -la"), Some("ls -la"));
    assert_eq!(bang_command("! git status "), Some("git status"));
    assert_eq!(bang_command("!"), Some(""));
    assert_eq!(bang_command("ls"), None);
    assert_eq!(bang_command("/run x"), None);
}

#[test]
fn bang_runs_command_in_a_pane() {
    let mut app = CrewApp::default();
    assert!(app.panes.is_empty());
    // `!cmd` spawns a pane running the command in the user's shell.
    assert!(!app.submit_input("!true".to_string()));
    assert_eq!(app.panes.len(), 1, "!cmd opens a command pane");
    // bare `!` is just a usage hint — no pane.
    assert!(!app.submit_input("!".to_string()));
    assert_eq!(app.panes.len(), 1, "bare ! opens no pane");
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
fn goal_slash_command_spawns_swarm_pane() {
    use crate::pane::PaneContent;
    let mut app = CrewApp::default();
    // `/goal <text>` plans then runs a swarm; bare `/goal` is just a usage hint.
    assert!(!app.submit_input("/goal".to_string()));
    assert!(app.panes.is_empty(), "bare /goal opens no pane");
    assert!(!app.submit_input("/goal ship the feature".to_string()));
    assert_eq!(app.panes.len(), 1);
    assert!(matches!(app.panes[0].content, PaneContent::Swarm(_)));
    assert_eq!(app.panes[0].title_text(), "swarm");
}

#[test]
fn batch_slash_command_spawns_swarm_pane_from_a_file() {
    use crate::pane::PaneContent;
    let mut app = CrewApp::default();
    // bare /batch → usage hint, no pane.
    assert!(!app.submit_input("/batch".to_string()));
    assert!(app.panes.is_empty(), "bare /batch opens no pane");

    let path = std::env::temp_dir().join("crew_batch_slash_test_jobs.txt");
    std::fs::write(&path, "first job\nsecond job\n").unwrap();
    assert!(!app.submit_input(format!("/batch {}", path.display())));
    assert_eq!(app.panes.len(), 1);
    assert!(matches!(app.panes[0].content, PaneContent::Swarm(_)));
    assert_eq!(app.panes[0].title_text(), "swarm");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn md_slash_command_opens_a_zoomed_markdown_pane() {
    use crate::pane::PaneContent;
    let mut app = CrewApp::default();
    // bare /md → usage hint, no pane.
    assert!(!app.submit_input("/md".to_string()));
    assert!(app.panes.is_empty(), "bare /md opens no pane");
    assert!(!app.zoomed);

    let path = std::env::temp_dir().join("crew_md_slash_test.md");
    std::fs::write(&path, "# Title\n").unwrap();
    assert!(!app.submit_input(format!("/md {}", path.display())));
    assert_eq!(app.panes.len(), 1);
    assert!(matches!(app.panes[0].content, PaneContent::Markdown(_)));
    assert!(app.zoomed, "/md spawns a zoomed pane");
    assert!(app.panes[0].title_text().ends_with(" · md"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn md_slash_command_missing_file_reports_status_and_opens_nothing() {
    let mut app = CrewApp::default();
    assert!(!app.submit_input("/md /nonexistent/path/for/crew/md/test.md".to_string()));
    assert!(app.panes.is_empty(), "unreadable file opens no pane");
    let msg = app
        .status
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default();
    assert!(!msg.is_empty(), "missing file must set a status error");
}

#[test]
fn closeall_closes_every_pane_and_refocuses_input() {
    let mut app = CrewApp::default();
    // /far twice → two panes.
    assert!(!app.submit_input("/far".to_string()));
    assert!(!app.submit_input("/far".to_string()));
    assert_eq!(app.panes.len(), 2);
    assert!(!app.submit_input("/closeall".to_string()));
    assert!(app.panes.is_empty(), "all panes closed");
    assert!(app.input.focused, "focus returns to the input bar");
}

#[test]
fn about_flashes_the_version() {
    let mut app = CrewApp::default();
    assert!(!app.submit_input("/about".to_string()));
    let msg = app
        .status
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default();
    assert!(
        msg.contains("crew v"),
        "about shows the version, got {msg:?}"
    );
    assert!(msg.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn clearall_with_no_terminals_reports_nothing() {
    let mut app = CrewApp::default();
    assert!(!app.submit_input("/far".to_string())); // a non-terminal pane
    assert!(!app.submit_input("/clearall".to_string()));
    let msg = app
        .status
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default();
    assert_eq!(msg, "nothing to clear");
}

#[test]
fn spawn_labeled_terminal_failure_is_shown_in_status() {
    let mut app = CrewApp::default();
    // A binary that cannot be exec'd → spawn errors; the failure must be visible
    // (it used to vanish to stderr, invisible in the GUI).
    app.spawn_labeled_terminal("crew-no-such-binary-xyzzy", &[], "x".to_string());
    assert!(app.panes.is_empty(), "a failed spawn opens no pane");
    let msg = app
        .status
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default();
    assert!(msg.contains("couldn't run"), "failure shown, got {msg:?}");
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
fn typing_clears_a_terminal_selection() {
    use crate::layout::Rect;
    use crate::pane::{Pane, PaneContent, TermPane};
    use crew_term::{GridSize, PtyTerm, TermModel};
    // A real shell pane (plain, no login flag — reliable under the test harness)
    // with an active mouse selection.
    let mut app = CrewApp::default();
    // Absolute shell path + an explicit, existing cwd so the spawn never depends
    // on $PATH or the process's (possibly test-mutated) working directory.
    let tmp = std::env::temp_dir();
    let pty =
        PtyTerm::spawn_in(GridSize { cols: 40, rows: 10 }, "/bin/sh", &[], Some(&tmp)).unwrap();
    let input = pty.writer();
    app.panes.push(Pane {
        content: PaneContent::Terminal(Box::new(TermPane {
            pty,
            input,
            cmd: None,
            cmd_since: None,
        })),
        grid: GridSize { cols: 40, rows: 10 },
        rect: Rect {
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
    });
    app.focused = 0;
    if let Some(PaneContent::Terminal(t)) = app.panes.get_mut(0).map(|p| &mut p.content) {
        t.pty.feed(b"hello world");
        t.pty.sel_start(0, 0, false);
        t.pty.sel_update(4, 0);
    }
    assert!(app.pane_selection_text(0).is_some(), "selection armed");
    // Typing into the focused terminal must clear the stale highlight.
    app.write_to_terminals(b"x");
    assert_eq!(app.pane_selection_text(0), None, "type clears selection");
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

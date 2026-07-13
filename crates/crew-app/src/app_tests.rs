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
        hidden: false,
        attention: None,
    }
}

/// A chat pane backed by an idle child process — only pane state is under
/// test, so `sh -c cat >/dev/null` stands in for the broker.
fn tests_chat_pane() -> crate::pane::Pane {
    use crate::chat::ChatPane;
    use crate::pane::{Pane, PaneContent};
    use crew_plugin::Plugin;
    use crew_term::GridSize;
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    Pane {
        content: PaneContent::Chat(ChatPane::new(plugin, "crew".into())),
        grid: GridSize { cols: 80, rows: 24 },
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
    }
}

#[test]
fn closing_chat_pane_closes_hive_companion() {
    let mut app = CrewApp::default();
    app.panes.push(tests_chat_pane());
    app.hive_plan(vec![]); // empty plan is a valid graph
    assert_eq!(app.panes.len(), 2);
    app.close_pane(0);
    assert!(app.panes.is_empty(), "companion closes with its chat");
}

#[test]
fn focusing_a_pane_clears_its_attention_but_not_others() {
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("a"));
    app.panes.push(tests_far_pane("b"));
    for p in &mut app.panes {
        p.activity = true;
        p.bell = true;
        crate::attention::raise(p, crate::notify::NotifyKind::Bell, 0);
    }
    app.focused = 0;
    app.mark_focused_seen();
    assert!(!app.panes[0].activity && !app.panes[0].bell);
    assert_eq!(app.panes[0].attention, None, "looking at it clears it");
    assert!(
        app.panes[1].attention.is_some(),
        "the unfocused pane keeps its marker"
    );
}

#[test]
fn input_bar_focus_keeps_the_attention_marker() {
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("a"));
    crate::attention::raise(&mut app.panes[0], crate::notify::NotifyKind::Bell, 0);
    app.input.focused = true;
    app.mark_focused_seen();
    assert!(
        app.panes[0].attention.is_some(),
        "typing in the bar isn't looking at the pane"
    );
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
    // Pre-Task-3 this asserted that ANY bare text with no terminal open hints
    // (it used to be written to nowhere, silently). Smart routing now spawns
    // a pane for a real command like `ls` instead — see
    // `bare_resolvable_command_spawns_with_no_idle_shell` for that case.
    // What still can't be silently dropped is unresolvable text: hint instead.
    // This variant covers Target::Other arising from having NO panes at all;
    // `bare_nonsense_with_no_shell_hints_instead_of_spawning` covers the other
    // way Target::Other arises — a focused pane that isn't a terminal.
    let mut app = CrewApp::default();
    assert!(!app.submit_input("definitely-not-a-command-xyz".to_string()));
    assert!(app.panes.is_empty(), "no junk pane spawned for nonsense");
    assert!(app.active_status().is_some());
}

/// Verdict::Executable + Target::Other (no idle shell focused) spawns a new
/// terminal pane running the command, end to end through `submit_input`.
#[test]
fn bare_resolvable_command_spawns_with_no_idle_shell() {
    use crate::pane::PaneContent;
    let mut app = CrewApp::default();
    assert!(app.panes.is_empty());
    // No panes at all → focused_target() is Target::Other; `ls` resolves to
    // Verdict::Executable, so route_bare says Spawn.
    assert!(!app.submit_input("ls".to_string()));
    assert_eq!(app.panes.len(), 1, "a real command spawns exactly one pane");
    assert!(
        matches!(app.panes[0].content, PaneContent::Terminal(_)),
        "spawned pane runs the command in a terminal"
    );
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
        hidden: false,
        attention: None,
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
fn reconcile_grid_keeps_hidden_panes_out() {
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("a"));
    app.panes.push(tests_far_pane("b"));
    app.focused = 0;
    app.input.focused = false;
    app.reconcile_grid();
    assert_eq!(app.grid.len(), 2);
    // Hiding a pane removes it from the grid: not a full tile, and — unlike
    // LRU demotion — not in the bottom strip either.
    app.panes[1].hidden = true;
    app.reconcile_grid();
    assert_eq!(app.grid.full(), &[0]);
    assert!(app.grid.minimized().is_empty(), "hidden ≠ LRU strip");
    // Repeated reconciles must not resurrect it.
    app.reconcile_grid();
    assert_eq!(app.grid.len(), 1);
}

#[test]
fn focusing_a_hidden_pane_restores_it() {
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("a"));
    app.panes.push(tests_far_pane("b"));
    app.panes[1].hidden = true;
    // Keyboard focus lands on the hidden pane (nav click, Cmd+N…): restore it.
    app.focused = 1;
    app.input.focused = false;
    app.reconcile_grid();
    assert!(!app.panes[1].hidden);
    assert_eq!(app.grid.full()[0], 1, "restored pane re-enters as MRU");
}

#[test]
fn input_bar_focus_does_not_restore_hidden_pane() {
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("a"));
    // The only pane is hidden and the input bar holds focus: `focused` still
    // points at the pane, but no pane is active — it must stay hidden.
    app.panes[0].hidden = true;
    app.focused = 0;
    app.input.focused = true;
    app.reconcile_grid();
    assert!(app.panes[0].hidden);
    assert_eq!(app.grid.len(), 0);
}

#[test]
fn closing_last_visible_pane_keeps_hidden_panes_tucked() {
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("tucked"));
    app.panes.push(tests_far_pane("open"));
    app.panes[0].hidden = true;
    app.focused = 1;
    app.input.focused = false;
    // Closing the only visible pane must NOT resurrect the minimized one:
    // focus falls to the input bar, and reconcile leaves it tucked away.
    app.close_pane(1);
    assert!(app.input.focused, "no visible pane left → input bar");
    app.reconcile_grid();
    assert!(app.panes[0].hidden, "minimized pane stays in the nav");
    assert_eq!(app.grid.len(), 0);
}

#[test]
fn closing_a_pane_moves_focus_to_a_visible_pane() {
    let mut app = CrewApp::default();
    for n in ["a", "b", "c"] {
        app.panes.push(tests_far_pane(n));
    }
    app.panes[0].hidden = true;
    app.focused = 1;
    app.input.focused = false;
    // Closing focused "b" leaves [a(hidden), c]: focus must skip the hidden
    // pane and land on "c" (now index 1), not restore "a".
    app.close_pane(1);
    assert_eq!(app.focused, 1);
    assert!(!app.input.focused);
    app.reconcile_grid();
    assert!(app.panes[0].hidden);
}

#[test]
fn pane_cycling_skips_hidden_panes() {
    let mut app = CrewApp::default();
    for n in ["a", "b", "c"] {
        app.panes.push(tests_far_pane(n));
    }
    app.panes[1].hidden = true;
    app.focused = 0;
    app.input.focused = false;
    // Cmd+] hops 0 → 2 (skipping hidden 1); again wraps 2 → 0; Cmd+[ back to 2.
    app.handle_super_chord("]");
    assert_eq!(app.focused, 2);
    app.handle_super_chord("]");
    assert_eq!(app.focused, 0);
    app.handle_super_chord("[");
    assert_eq!(app.focused, 2);
    app.reconcile_grid();
    assert!(
        app.panes[1].hidden,
        "cycling never restores a minimized pane"
    );
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

#[test]
fn star_broadcast_with_no_terminals_hints() {
    let mut app = CrewApp::default();
    app.submit_input("* echo hi".into());
    let status = app
        .status
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default();
    assert!(status.contains("no terminals"), "got: {status}");
}

#[test]
fn apply_config_resumes_saved_mode_and_pins_fixed_themes() {
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    let mut cfg = app.config.clone();
    cfg.theme = Some("random-light".to_string());
    app.apply_config(cfg);
    assert_eq!(crew_theme::mode(), Some(crew_theme::RandomMode::Light));
    assert!(!crew_theme::current_id().is_dark());
    let mut cfg = app.config.clone();
    cfg.theme = Some("graphite".to_string());
    app.apply_config(cfg);
    assert_eq!(crew_theme::mode(), None);
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::Graphite);
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    );
}

#[test]
fn bare_nonsense_with_no_shell_hints_instead_of_spawning() {
    // Same unresolvable-text outcome as `submit_without_a_shell_hints`, but
    // Target::Other arises the other way here: a focused pane that exists but
    // isn't a terminal (vs. no panes at all).
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("files")); // focused pane is Far, not a terminal
    app.focused = 0;
    app.submit_input("definitely-not-a-command-xyz".into());
    assert_eq!(app.panes.len(), 1, "no junk pane spawned");
    let status = app
        .status
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default();
    assert!(status.contains("not a command"), "got: {status}");
}

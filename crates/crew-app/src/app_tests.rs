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
        glide: crate::glide::Glide::default(),
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
        born_ms: crate::anim::now_ms(),
    }
}

fn tests_chat_pane() -> crate::pane::Pane {
    use crate::chat::ChatPane;
    use crate::pane::{Pane, PaneContent};
    use crew_plugin::Plugin;
    use crew_term::GridSize;
    // An idle child stands in for the broker; only pane state is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    Pane {
        glide: crate::glide::Glide::default(),
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
        born_ms: crate::anim::now_ms(),
    }
}

fn chat_pane_compact(app: &CrewApp, i: usize) -> bool {
    match &app.panes[i].content {
        crate::pane::PaneContent::Chat(c) => c.compact_view,
        _ => unreachable!("expected a chat pane"),
    }
}

#[test]
fn toggle_compact_focused_flips_the_focused_chat_pane_and_back() {
    // This is the effectful half of the Ctrl+O global intercept (keys.rs) —
    // the chord-matching half is `is_compact_chord`, tested in keys.rs.
    let mut app = CrewApp::default();
    app.panes.push(tests_chat_pane());
    app.focused = 0;
    assert!(!chat_pane_compact(&app, 0), "compact_view starts off");

    assert!(
        app.toggle_compact_focused(),
        "found a chat pane at the focused index"
    );
    assert!(chat_pane_compact(&app, 0), "first Ctrl+O turns it on");

    assert!(app.toggle_compact_focused());
    assert!(
        !chat_pane_compact(&app, 0),
        "second Ctrl+O restores the full transcript"
    );
}

#[test]
fn toggle_compact_focused_is_a_noop_on_a_non_chat_pane() {
    // Terminal (and other) panes must fall through untouched so Ctrl+O still
    // reaches the PTY as a raw byte instead of being swallowed here.
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("a"));
    app.focused = 0;

    assert!(!app.toggle_compact_focused());
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
fn md_slash_command_opens_a_document_window() {
    let mut app = CrewApp::default();
    // bare /md → usage hint, no pane and no window.
    assert!(!app.submit_input("/md".to_string()));
    assert!(app.panes.is_empty(), "bare /md opens no pane");
    assert!(app.pending_docs.is_empty(), "…and asks for no window");
    assert!(!app.zoomed);

    let path = std::env::temp_dir().join("crew_md_slash_test.md");
    std::fs::write(&path, "# Title\n").unwrap();
    assert!(!app.submit_input(format!("/md {}", path.display())));
    // `/md` is the markdown-shaped door: a document in a window of its own,
    // where it can be edited. The window itself is opened on the next tick,
    // which is the only place an active event loop exists.
    assert_eq!(app.pending_docs, vec![path.clone()]);
    assert!(app.panes.is_empty(), "and no pane in the grid");
    let _ = std::fs::remove_file(&path);
}

/// `/view` is the same viewer under its own name — `/md` is kept only as an
/// alias (see `dispatch::run_slash_command`).
#[test]
fn view_slash_command_opens_the_same_viewer_as_md() {
    use crate::pane::PaneContent;
    let mut app = CrewApp::default();
    assert!(!app.submit_input("/view".to_string()));
    assert!(app.panes.is_empty(), "bare /view opens no pane");

    let path = std::env::temp_dir().join("crew_view_slash_test.txt");
    std::fs::write(&path, "hello\n").unwrap();
    assert!(!app.submit_input(format!("/view {}", path.display())));
    assert_eq!(app.panes.len(), 1);
    assert!(matches!(app.panes[0].content, PaneContent::View(_)));
    assert!(app.zoomed, "/view spawns a zoomed pane");
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
    // It asks first — a closed pane takes its scrollback and its agent with
    // it — and the same command again is the answer.
    assert!(!app.submit_input("/closeall".to_string()));
    assert_eq!(app.panes.len(), 2, "the first /closeall closed something");
    assert!(!app.submit_input("/closeall".to_string()));
    assert!(app.panes.is_empty(), "all panes closed");
    assert!(app.input.focused, "focus returns to the input bar");
}

#[test]
fn about_opens_what_this_build_changed() {
    let mut app = CrewApp::default();
    let before = app.panes.len();
    assert!(!app.submit_input("/about".to_string()));
    // `/about` opens the changelog rather than flashing a version number:
    // the number is only useful as a way to find out what changed.
    assert_eq!(app.panes.len(), before + 1, "/about opened no pane");
    assert!(matches!(
        app.panes.last().map(|p| &p.content),
        Some(crate::pane::PaneContent::View(_))
    ));
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
    let base = crate::cwd::canonical(&std::env::temp_dir());
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
// Depends on `ls` existing on PATH, which is a POSIX assumption — on
// Windows this asserts the platform's command set, not crew's routing.
#[cfg(unix)]
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
    let (a, b) = (crate::cwd::canonical(&a), crate::cwd::canonical(&b));

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

// Drives a real PTY running a POSIX shell: Unix-only by construction.
// Windows has no `sh`, so the spawn fails on a detail that says nothing
// about the behaviour under test.
#[cfg(unix)]
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
        glide: crate::glide::Glide::default(),
        content: PaneContent::Terminal(Box::new(TermPane {
            pty,
            input,
            cmd: None,
            cmd_since: None,
            tail: Default::default(),
            read_at: 0,
            spans: Default::default(),
            trail: Default::default(),
            images: Default::default(),
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
        born_ms: crate::anim::now_ms(),
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
    // A retired theme name: it must resolve to its nearest survivor rather
    // than silently resetting to the default (see `from_name`).
    cfg.theme = Some("graphite".to_string());
    app.apply_config(cfg);
    assert_eq!(crew_theme::mode(), None);
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperDark);
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

/// Whether the chat pane at `i` still holds an open key prompt.
fn has_keyentry(app: &CrewApp, i: usize) -> bool {
    match &app.panes[i].content {
        crate::pane::PaneContent::Chat(c) => c.keyentry.is_some(),
        _ => unreachable!("expected a chat pane"),
    }
}

fn open_keyentry(app: &mut CrewApp, i: usize) {
    match &mut app.panes[i].content {
        crate::pane::PaneContent::Chat(c) => {
            c.keyentry = Some(crate::keyentry::KeyEntry::new("ANTHROPIC_API_KEY".into()));
        }
        _ => unreachable!("expected a chat pane"),
    }
}

/// Whether the chat pane at `i` still holds a live browser sign-in.
fn has_oauth(app: &CrewApp, i: usize) -> bool {
    match &app.panes[i].content {
        crate::pane::PaneContent::Chat(c) => c.oauth.is_some(),
        _ => unreachable!("expected a chat pane"),
    }
}

/// Open an OpenRouter prompt with a sign-in in flight behind it, as accepting
/// the dimmed row does. Hands back the sender so the test can decide whether
/// the worker's send still has a receiver to reach.
fn open_oauth_keyentry(
    app: &mut CrewApp,
    i: usize,
) -> std::sync::mpsc::Sender<crate::oauth::OauthOutcome> {
    let (tx, rx) = std::sync::mpsc::channel();
    match &mut app.panes[i].content {
        crate::pane::PaneContent::Chat(c) => {
            let mut entry = crate::keyentry::KeyEntry::new("OPENROUTER_API_KEY".into());
            entry.set_waiting(true);
            c.keyentry = Some(entry);
            c.oauth = Some(rx);
        }
        _ => unreachable!("expected a chat pane"),
    }
    tx
}

/// The pane at `i` as a `ChatPane`.
fn chat(app: &mut CrewApp, i: usize) -> &mut crate::chat::ChatPane {
    match &mut app.panes[i].content {
        crate::pane::PaneContent::Chat(c) => c,
        _ => unreachable!("expected a chat pane"),
    }
}

/// What the key prompt at `i` is drawing right now: `(mask glyphs, hint?)`.
/// The buffer itself is private and stays that way — this is the only view of
/// it anything, test included, is allowed.
fn card_state(app: &CrewApp, i: usize) -> (usize, bool) {
    match &app.panes[i].content {
        crate::pane::PaneContent::Chat(c) => {
            let cells = c.keyentry.as_ref().expect("a prompt is open").card(60);
            let drawn: String = cells.iter().map(|c| c.c).collect();
            (
                cells.iter().filter(|c| c.c == '•').count(),
                drawn.contains("waiting for browser"),
            )
        }
        _ => unreachable!("expected a chat pane"),
    }
}

/// The last note in the pane at `i`, or `""`.
fn last_note(app: &CrewApp, i: usize) -> String {
    match &app.panes[i].content {
        crate::pane::PaneContent::Chat(c) => c
            .messages
            .last()
            .map(|m| m.text.clone())
            .unwrap_or_default(),
        _ => unreachable!("expected a chat pane"),
    }
}

/// HIDDEN IS NOT DISMISSED. This is the ordinary end of the flow: the browser
/// says "you can close this tab", the user clicks back into crew, and the
/// activating click lands on the input bar. Cancelling there threw away a
/// sign-in the user had just completed — silently, and leaving a real key
/// minted on their OpenRouter account for them to find and revoke by hand.
#[test]
fn a_prompt_hidden_by_the_input_bar_keeps_its_sign_in_and_still_completes() {
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("credentials.json");
    let mut app = CrewApp::default();
    app.panes.push(tests_chat_pane());
    app.focused = 0;
    let tx = open_oauth_keyentry(&mut app, 0);
    // Half a key typed by hand before the browser came back.
    for c in "sk-half".chars() {
        chat(&mut app, 0)
            .keyentry
            .as_mut()
            .unwrap()
            .key(&crate::chatkeys::ChatInput::Char(c));
    }

    app.input.focused = true; // the click that brings crew back to the front
    app.close_hidden_keyentry();

    assert!(
        has_oauth(&app, 0),
        "coming back from the browser must not cancel the sign-in"
    );
    assert!(has_keyentry(&app, 0), "the prompt comes back with the pane");
    assert_eq!(
        card_state(&app, 0),
        (0, true),
        "hidden forgets what was typed and goes back to the browser hint"
    );

    // The worker still has somewhere to send, and the key still lands.
    tx.send(crate::oauth::OauthOutcome::Key("sk-or-v1-fake".into()))
        .expect("the worker's send must still reach the pane");
    assert!(app.drain_oauth_into(Some(&creds)), "the outcome lands");
    assert_eq!(
        crew_plugin::credentials::load_from(&creds)
            .keys
            .get("OPENROUTER_API_KEY")
            .map(String::as_str),
        Some("sk-or-v1-fake"),
        "a completed sign-in must be stored, not dropped on the floor"
    );
    assert!(
        !has_keyentry(&app, 0),
        "the prompt closes once it is answered"
    );
}

/// The other two ways the card stops being drawn are the same story: neither
/// is the user abandoning the sign-in.
#[test]
fn switching_panes_or_opening_help_does_not_cancel_the_browser_sign_in() {
    let mut app = CrewApp::default();
    app.panes.push(tests_chat_pane());
    app.panes.push(tests_chat_pane());
    app.focused = 0;
    let tx = open_oauth_keyentry(&mut app, 0);
    app.focused = 1;
    app.close_hidden_keyentry();
    assert!(has_oauth(&app, 0), "another pane's focus is not a cancel");

    open_oauth_keyentry(&mut app, 1);
    app.help_open = true;
    app.close_hidden_keyentry();
    assert!(
        has_oauth(&app, 1),
        "the help overlay is not a cancel either"
    );

    // Still live, both of them: the outcome can still be delivered.
    assert!(tx
        .send(crate::oauth::OauthOutcome::Failed("late".into()))
        .is_ok());
    assert!(app.drain_oauth_into(None));
}

/// A prompt with no flow behind it is still discarded the moment it stops
/// being drawn — that is the invariant the hidden case must not weaken.
#[test]
fn a_hidden_prompt_with_no_sign_in_is_still_discarded() {
    let mut app = CrewApp::default();
    app.panes.push(tests_chat_pane());
    app.focused = 0;
    open_keyentry(&mut app, 0); // ANTHROPIC_API_KEY: no browser flow at all
    app.input.focused = true;
    app.close_hidden_keyentry();
    assert!(
        !has_keyentry(&app, 0),
        "a prompt the frame will not draw must not survive holding a secret"
    );
}

/// DISMISSED, not hidden: Escape ends the flow, says so, and nothing can be
/// stored from it afterwards.
#[test]
fn escaping_the_prompt_cancels_the_sign_in_visibly_and_stores_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("credentials.json");
    let mut app = CrewApp::default();
    app.panes.push(tests_chat_pane());
    app.focused = 0;
    let tx = open_oauth_keyentry(&mut app, 0);

    chat(&mut app, 0).on_input(crate::chatkeys::ChatInput::Close, std::path::Path::new("."));

    assert!(!has_keyentry(&app, 0), "escape closes the prompt");
    assert!(!has_oauth(&app, 0), "escape cancels the sign-in behind it");
    assert!(
        last_note(&app, 0).contains("cancelled"),
        "a cancelled sign-in must never be silent: {:?}",
        last_note(&app, 0)
    );
    // Dropping the receiver is what ends the worker thread, so a key that was
    // already in flight has nowhere to land.
    assert!(
        tx.send(crate::oauth::OauthOutcome::Key("sk-or-v1-fake".into()))
            .is_err(),
        "the worker's send must fail once the prompt is dismissed"
    );
    assert!(!app.drain_oauth_into(Some(&creds)), "nothing left to drain");
    assert!(
        !creds.exists(),
        "a dismissed prompt must store no key at all"
    );
}

/// The outcome belongs to the pane that started the flow, whether or not you
/// are still looking at it. Polling only the focused pane stranded it: the
/// user approved, nothing happened, and the note turned up minutes later.
#[test]
fn a_sign_in_lands_in_its_own_pane_even_while_another_is_focused() {
    let mut app = CrewApp::default();
    app.panes.push(tests_chat_pane());
    app.panes.push(tests_chat_pane());
    app.focused = 0;
    let tx = open_oauth_keyentry(&mut app, 0);
    app.focused = 1; // the user wandered off to another pane

    tx.send(crate::oauth::OauthOutcome::Failed("access_denied".into()))
        .unwrap();
    assert!(app.drain_oauth(), "the outcome must land on this tick");

    assert!(!has_oauth(&app, 0), "the receiver is spent and cleared");
    match &app.panes[0].content {
        crate::pane::PaneContent::Chat(c) => {
            let note = c
                .messages
                .last()
                .expect("a note reached pane 0")
                .text
                .clone();
            assert!(note.contains("access_denied"), "got: {note:?}");
        }
        _ => unreachable!("expected a chat pane"),
    }
    match &app.panes[1].content {
        crate::pane::PaneContent::Chat(c) => assert!(
            c.messages.is_empty(),
            "the note belongs to the pane that started the flow, not the focused one"
        ),
        _ => unreachable!("expected a chat pane"),
    }
}

/// Remote-supplied callback text must not reach a pane note at full length.
#[test]
fn a_failure_note_clamps_text_that_came_from_the_callback() {
    let mut app = CrewApp::default();
    app.panes.push(tests_chat_pane());
    app.focused = 0;
    let tx = open_oauth_keyentry(&mut app, 0);
    tx.send(crate::oauth::OauthOutcome::Failed("e".repeat(5_000)))
        .unwrap();
    assert!(app.drain_oauth());
    match &app.panes[0].content {
        crate::pane::PaneContent::Chat(c) => {
            let note = &c.messages.last().expect("a note").text;
            assert!(note.chars().count() < 200, "note ran to {}", note.len());
        }
        _ => unreachable!("expected a chat pane"),
    }
}

/// Focusing the input bar is a plain mouse click (`focus_at_cursor`), and
/// `render.rs` draws the key card only while the bar is NOT focused. The
/// prompt would otherwise survive open-but-invisible, still holding a
/// half-typed secret — and, because keys no longer reach `ChatPane::on_input`
/// at all, the rest of the key would be typed into the input bar in plaintext
/// and persisted to the on-disk command history on Enter.
#[test]
fn focusing_the_input_bar_discards_an_open_key_prompt() {
    let mut app = CrewApp::default();
    app.panes.push(tests_chat_pane());
    app.focused = 0;
    open_keyentry(&mut app, 0);

    app.close_hidden_keyentry();
    assert!(
        has_keyentry(&app, 0),
        "the prompt stays while its own pane is the one being drawn"
    );

    app.input.focused = true; // a click on the input bar
    app.close_hidden_keyentry();
    assert!(
        !has_keyentry(&app, 0),
        "a prompt the frame will not draw must not survive holding a secret"
    );
}

/// The same invariant for the other two ways the card stops being drawn:
/// focus moving to a different pane (`render.rs` only draws the card for
/// `self.focused`), and the help overlay, which returns before the card is
/// ever pushed.
#[test]
fn switching_panes_or_opening_help_discards_an_open_key_prompt() {
    let mut app = CrewApp::default();
    app.panes.push(tests_chat_pane());
    app.panes.push(tests_chat_pane());
    app.focused = 0;
    open_keyentry(&mut app, 0);
    app.focused = 1; // clicked the other pane
    app.close_hidden_keyentry();
    assert!(!has_keyentry(&app, 0), "an unfocused pane's prompt is gone");

    open_keyentry(&mut app, 1);
    app.help_open = true;
    app.close_hidden_keyentry();
    assert!(
        !has_keyentry(&app, 1),
        "the help overlay covers the card, so the prompt must not linger under it"
    );
}

/// A note decided at startup must survive to a frame. Status messages expire
/// after three seconds and a cold launch takes minutes to draw anything, so
/// the version-change announcement is held rather than flashed — otherwise it
/// would be gone on exactly the launch it exists for.
#[test]
fn a_pending_note_waits_for_a_frame_and_flashes_once() {
    let mut app = CrewApp {
        pending_note: Some("updated to crew 9.9.9".into()),
        ..CrewApp::default()
    };
    assert!(app.status.is_none(), "nothing flashes before a frame");

    // What the redraw arm does: take it, then flash it.
    if let Some(n) = app.pending_note.take() {
        app.set_status(n);
    }
    let shown = app
        .status
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default();
    assert!(shown.contains("9.9.9"), "{shown}");
    assert!(app.pending_note.is_none(), "a note must flash exactly once");
}

// --- dismissal ghosts -------------------------------------------------------

/// Closing a pane leaves its frame behind to collapse. The pane itself must be
/// gone immediately — focus clamping, the grid LRU and the nav rows all read
/// `panes`, and a pane lingering there would still be interactive.
#[test]
fn closing_a_pane_leaves_a_ghost_but_not_the_pane() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("p"));
    app.panes.push(tests_far_pane("p"));
    app.close_pane(0);
    assert_eq!(app.panes.len(), 1, "the pane must not linger");
    assert_eq!(app.ghosts.len(), 1, "its frame should still be collapsing");
    assert_eq!(app.ghosts[0].exit, crate::ghost::Exit::Closed);
}

/// Minimize means "it went into the nav", so its ghost travels that way — the
/// two dismissals have to be distinguishable or they read as the same gesture.
#[test]
fn minimizing_leaves_a_ghost_headed_for_the_nav() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("p"));
    app.minimize_pane(0);
    assert!(app.panes[0].hidden, "the pane keeps running, just hidden");
    assert_eq!(app.ghosts.len(), 1);
    assert_eq!(app.ghosts[0].exit, crate::ghost::Exit::Minimized);
}

/// Ghosts are bounded by their own timelines: nothing accumulates across a
/// session of opening and closing panes.
#[test]
fn ghosts_do_not_accumulate() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let mut app = CrewApp::default();
    for _ in 0..5 {
        app.panes.push(tests_far_pane("p"));
        app.close_pane(0);
    }
    assert_eq!(app.ghosts.len(), 5, "all still collapsing at t=now");
    crate::ghost::prune(&mut app.ghosts, crate::anim::now_ms() + 10_000);
    assert!(app.ghosts.is_empty(), "every ghost must expire");
}

/// Coming back out of the nav is an arrival: the pane assembles exactly as a
/// new one does, rather than snapping back into the grid.
#[test]
fn restoring_a_minimized_pane_re_assembles_it() {
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("p"));
    app.panes[0].hidden = true;
    app.panes[0].born_ms = 0;
    app.focused = 0;
    app.input.focused = false;
    app.reconcile_grid();
    assert!(!app.panes[0].hidden);
    assert!(app.panes[0].born_ms > 0, "birth clock should be re-stamped");
}

// --- the idle invariant -----------------------------------------------------

/// `anim.rs` opens by promising that an idle crew never repaints. Every
/// animation added since 0.8.0 is bounded specifically so this stays true —
/// which is only worth anything if something checks it.
#[test]
fn a_settled_app_asks_for_no_frames() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("p"));
    app.panes[0].born_ms = 0;
    // Long after everything could conceivably have finished.
    let late = crate::anim::now_ms() + 60_000;
    assert!(
        !app.wants_animation_frame(late),
        "an idle crew asked for another frame"
    );
}

/// Reduce-motion is not "the same animations, faster": at `off` nothing is
/// scheduled at all, from the very first frame.
#[test]
fn motion_off_schedules_nothing_even_immediately_after_events() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Off);
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("a"));
    app.panes.push(tests_far_pane("b"));
    let now = crate::anim::now_ms();
    app.panes[0].born_ms = now;
    app.focus_anim = crate::ease::Timeline::start(now, 300, crate::motion::level());
    app.zoom_anim = crate::ease::Timeline::start(now, 300, crate::motion::level());
    app.close_pane(1);
    assert!(
        !app.wants_animation_frame(now),
        "off must schedule nothing, not merely finish quickly"
    );
    crate::motion::set_level(crate::motion::MotionLevel::Full);
}

/// The other half of the contract: at full motion a fresh event DOES ask for
/// frames. Without this, the test above would pass on an app that never
/// animated at all.
#[test]
fn a_fresh_event_asks_for_frames_at_full_motion() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("p"));
    let now = crate::anim::now_ms();
    app.panes[0].born_ms = 0;
    app.focus_anim = crate::ease::Timeline::start(now, 300, crate::motion::level());
    assert!(app.wants_animation_frame(now), "focus travel must animate");
}

/// Every animation is bounded — a ghost, a focus travel and a spawn all end.
/// A timeline that never settled would keep the predicate true forever, which
/// is the one failure mode that costs battery rather than pixels.
#[test]
fn every_animation_terminates() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("a"));
    app.panes.push(tests_far_pane("b"));
    let now = crate::anim::now_ms();
    app.panes[0].born_ms = now;
    app.focus_anim = crate::ease::Timeline::start(now, 300, crate::motion::level());
    app.zoom_anim = crate::ease::Timeline::start(now, 300, crate::motion::level());
    app.minimize_pane(1);
    assert!(app.wants_animation_frame(now), "should be busy right now");
    assert!(
        !app.wants_animation_frame(now + 30_000),
        "something is still animating half a minute later"
    );
}

// --- the CRT ignition sweep --------------------------------------------------

/// Gaining focus on a CRT theme fires a one-shot ignition sweep that outlives
/// the 260ms bracket travel — and then settles: once the sweep has run its
/// ~600ms the app stops asking for frames again (done-criterion #5: idle
/// converges to a static frame, ignition included).
#[test]
fn crt_ignition_asks_for_frames_then_settles() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    crew_theme::set_theme(crew_theme::ThemeId::CrtGreen);
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("p"));
    app.panes[0].born_ms = 0;
    let now = crate::anim::now_ms();
    // Pretend the last frame drew focus elsewhere, as any focus change does.
    app.focus_drawn = 1;
    app.focus_fx(now);
    assert!(
        app.wants_animation_frame(now + 400),
        "the ignition must still burn after the 260ms bracket travel ends"
    );
    assert!(
        !app.wants_animation_frame(now + 30_000),
        "a finished ignition must stop requesting frames"
    );
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

/// The reduce-motion contract holds for ignition too: at `off` the sweep is
/// born settled — one final-state frame, zero scheduled redraws.
#[test]
fn motion_off_births_the_ignition_settled() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Off);
    crew_theme::set_theme(crew_theme::ThemeId::CrtGreen);
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("p"));
    app.panes[0].born_ms = 0;
    // `focus_fx` scales by the CONFIG's motion level (as `build_frame` always
    // has for the bracket travel), so pin the config alongside the global.
    app.config.motion = "off".into();
    let now = crate::anim::now_ms();
    app.focus_drawn = 1;
    app.focus_fx(now);
    assert!(
        !app.ignite_anim.live(now),
        "off must be born settled, not merely finish quickly"
    );
    assert!(!app.wants_animation_frame(now));
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

/// Paper themes never ignite: the sweep changes no paper pixel, so spawning
/// it there would spend 600ms of redraws drawing the same frame.
#[test]
fn focusing_ignites_the_ring_on_every_theme() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    // The inverse of what this asserted before. Ignition is the gradient ring lighting up as
    // focus lands, and it used to be reserved for the two modern themes; now every palette has
    // a ring, so every palette lights it. The bracket travel still runs alongside it, which is
    // the part that was never about the ring.
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let mut app = CrewApp::default();
        app.panes.push(tests_far_pane("p"));
        app.panes[0].born_ms = 0;
        let now = crate::anim::now_ms();
        app.focus_drawn = 1;
        app.focus_fx(now);
        assert!(
            app.ignite_anim.live(now + 1),
            "{} does not ignite its ring on focus",
            id.as_str()
        );
        assert!(
            app.focus_anim.live(now + 1),
            "{} lost the bracket travel",
            id.as_str()
        );
    }
}

/// Cmd+F did nothing at all outside a chat pane — including in the pane kind
/// crew has most of. It opens the bar's `/find`, typed and waiting.
#[test]
fn cmd_f_outside_a_chat_pane_opens_find_in_the_bar() {
    let mut app = CrewApp::default();
    app.panes.push(tests_far_pane("far"));
    app.focused = 0;
    app.handle_super_chord("f");
    assert_eq!(app.input.text, "/find ");
    assert!(app.input.focused);
    // A chat pane keeps its own in-transcript find rather than the bar's.
    let mut chat = CrewApp::default();
    chat.panes.push(tests_chat_pane());
    chat.focused = 0;
    chat.handle_super_chord("f");
    assert!(chat.input.text.is_empty(), "{:?}", chat.input.text);
}

/// Every `.rs` file of this crate, as `(name, source)`.
///
/// `include_str!` cannot take a glob, and a test that reads the source tree
/// through `std::fs` needs a root: `CARGO_MANIFEST_DIR` is the one path cargo
/// guarantees, and it is what the docs-parity tests already walk from.
pub(crate) fn crate_sources() -> Vec<(String, String)> {
    fn walk(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                if let Ok(s) = std::fs::read_to_string(&p) {
                    let name = p
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    out.push((name, s));
                }
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    walk(&root, &mut out);
    out
}

/// Every `#[test] fn name() { … }` in `src`, as `(name, body)`. Brace-matched
/// rather than line-scanned, so a test containing a nested block or a string
/// with a brace in it still yields its whole body and nothing after it.
pub(crate) fn test_bodies(src: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = src[at..].find("#[test]") {
        let i = at + i;
        let Some(fname) = src[i..].find("fn ") else {
            break;
        };
        let head = i + fname + 3;
        let Some(paren) = src[head..].find('(') else {
            break;
        };
        let name = src[head..head + paren].trim().to_string();
        let Some(open) = src[head..].find('{') else {
            break;
        };
        let mut depth = 1;
        let mut j = head + open + 1;
        for c in src[j..].chars() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            j += c.len_utf8();
            if depth == 0 {
                break;
            }
        }
        out.push((name, src[head + open + 1..j.saturating_sub(1)].to_string()));
        at = j;
    }
    out
}

/// Every test that reads the process-wide theme is serialised against every
/// test that changes it.
///
/// The theme, its accent and its gradient poles are process globals: a test
/// that paints cells from one palette and then compares them against
/// `theme()` is comparing against whatever palette is in force *now*, which
/// under a parallel runner is not necessarily the one it painted with. That
/// is not a hypothetical — `alert_toasts_border_in_the_bell_color` and
/// `version_stamp_present` both failed exactly this way (the second only on
/// Windows CI, where the runner schedules differently), and neither took the
/// guard.
///
/// Read out of this crate's own sources, because the alternative is
/// remembering: a new test that compares a colour is the easiest thing in the
/// world to write without a guard, and it passes locally every time until it
/// does not.
#[test]
fn every_test_that_reads_the_theme_takes_the_guard() {
    let mut unguarded: Vec<String> = Vec::new();
    for (path, src) in crate_sources() {
        for (name, body) in test_bodies(&src) {
            let reads = [
                "crew_theme::theme()",
                "crew_theme::current_id",
                "poleshift::",
                "theme().",
            ]
            .iter()
            .any(|needle| body.contains(needle));
            // A helper the test calls may hold it instead — the guard is not
            // re-entrant, so a test that both takes it AND calls such a
            // helper would deadlock rather than race.
            let guarded = body.contains("theme_test_guard") || body.contains("let _g = guard()");
            if reads && !guarded {
                unguarded.push(format!("{path}::{name}"));
            }
        }
    }
    assert!(
        unguarded.is_empty(),
        "these tests read the theme without serialising against the tests that change it:\n  {}",
        unguarded.join("\n  ")
    );
}

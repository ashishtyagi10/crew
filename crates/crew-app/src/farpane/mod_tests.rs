use super::ask::AskState;
use super::cmdhist::CmdHistory;
use super::keys::{
    accept_ghost, activate, ascend, escape_cmdline, history_next, history_prev, move_sel,
    tab_complete,
};
use super::run::{run_cmdline, submit_ask};
use super::{FarAction, FarPane, Side};

/// A FarPane rooted at a unique temp dir containing one subdirectory and one
/// file. `key` keeps each test isolated so the parallel runner can't race on a
/// shared path.
fn fixture(key: &str) -> (std::path::PathBuf, FarPane) {
    let base = std::env::temp_dir().join(format!("crew_far_mod_{key}"));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("sub")).unwrap();
    std::fs::write(base.join("f.txt"), b"x").unwrap();
    let pane = FarPane::new(base.clone());
    (base, pane)
}

#[test]
fn starts_active_left_on_given_dir() {
    let (base, p) = fixture("start");
    assert!(matches!(p.active, Side::Left));
    assert_eq!(p.left.cwd, base);
    assert_eq!(p.right.cwd, base);
    // ".." + "sub/" + "f.txt"
    assert_eq!(p.left.entries.len(), 3);
}

#[test]
fn tab_switches_active_panel() {
    let (_b, mut p) = fixture("tab");
    p.active = Side::Right;
    move_sel(&mut p, 1); // moves the RIGHT panel, not the left
    assert_eq!(p.right.sel, 1);
    assert_eq!(p.left.sel, 0);
}

#[test]
fn enter_descends_into_dir_and_back() {
    let (base, mut p) = fixture("descend");
    // entries[1] is "sub/" (dirs sort before files, after "..")
    p.left.sel = 1;
    activate(&mut p);
    assert_eq!(p.left.cwd, base.join("sub"));
    // the child dir has a ".." entry to climb back out
    assert!(p.left.entries.iter().any(|e| e.is_parent));
    ascend(&mut p);
    assert_eq!(p.left.cwd, base);
}

#[test]
fn command_line_runs_in_active_panel_dir_without_new_pane() {
    let (base, mut p) = fixture("cmdline");
    // Run a command from the right panel (which points at a subdir): it must
    // execute there — in place, no pane spawn — and reload when it finishes.
    p.active = Side::Right;
    p.right.cwd = base.join("sub");
    p.cmdline = "touch made-here".into();
    assert!(matches!(run_cmdline(&mut p), FarAction::Status(_)));
    assert!(p.cmdline.is_empty(), "running consumes the command line");
    assert!(p.is_busy(), "command runs on a worker thread");
    // Wait for the worker to finish and the poll to pick it up.
    let start = std::time::Instant::now();
    let status = loop {
        if let Some(msg) = p.poll_cmd() {
            break msg;
        }
        assert!(start.elapsed().as_secs() < 10, "command never finished");
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    assert!(status.contains("ok"), "status: {status}");
    assert!(!p.is_busy());
    assert!(
        base.join("sub/made-here").is_file(),
        "ran in the RIGHT panel's dir"
    );
    // The reload made the new file visible in the right panel's listing.
    assert!(p.right.entries.iter().any(|e| e.name == "made-here"));
}

#[test]
fn cd_moves_only_the_active_panel() {
    let (base, mut p) = fixture("cdactive");
    p.cmdline = "cd sub".into();
    assert!(matches!(run_cmdline(&mut p), FarAction::Status(_)));
    assert_eq!(
        p.left.cwd,
        base.join("sub"),
        "active (left) panel navigated"
    );
    assert_eq!(p.right.cwd, base, "inactive panel untouched");
    assert!(!p.is_busy(), "cd is handled in-process, nothing spawns");
}

#[test]
fn cd_to_a_missing_directory_reports_and_stays() {
    let (base, mut p) = fixture("cdmissing");
    p.cmdline = "cd nope".into();
    match run_cmdline(&mut p) {
        FarAction::Status(msg) => assert!(msg.contains("not a directory"), "{msg}"),
        _ => panic!("expected a status"),
    }
    assert_eq!(p.left.cwd, base);
}

#[test]
fn whitespace_command_line_does_not_run() {
    let (_b, mut p) = fixture("blankcmd");
    p.cmdline = "   ".into();
    assert!(matches!(run_cmdline(&mut p), FarAction::Status(_)));
}

#[test]
fn move_sel_clamps_to_bounds() {
    let (_b, mut p) = fixture("enter");
    move_sel(&mut p, -5);
    assert_eq!(p.left.sel, 0);
    move_sel(&mut p, 99);
    assert_eq!(p.left.sel, p.left.entries.len() - 1);
}

#[test]
fn scroll_moves_active_cursor_clamped() {
    let (_b, mut p) = fixture("scroll");
    p.scroll(-99); // wheel down → toward bottom
    assert_eq!(p.left.sel, p.left.entries.len() - 1);
    p.scroll(99); // wheel up → toward top
    assert_eq!(p.left.sel, 0);
}

#[test]
fn new_pane_starts_with_empty_completion_and_scan_state() {
    // `bins` is now the session-wide cache (shared across every FarPane in
    // the process, see `shared_bins`), so it may already be populated by
    // another test's scan by the time this runs under the parallel test
    // runner — only the per-pane `bins_scan_started` guard is meaningful to
    // assert here.
    let (_b, p) = fixture("newstate");
    assert!(p.complete.is_none());
    assert!(
        !p.bins_scan_started,
        "a fresh pane hasn't started its own scan"
    );
}

#[test]
fn tab_completes_a_unique_path_candidate_with_trailing_space() {
    let (base, mut p) = fixture("tabunique");
    std::fs::write(base.join("readme.md"), b"x").unwrap();
    p.cmdline = "cat read".into();
    tab_complete(&mut p);
    assert_eq!(p.cmdline, "cat readme.md ");
    assert!(p.complete.is_none(), "single match doesn't start a cycle");
}

#[test]
fn tab_completes_a_directory_without_a_trailing_space() {
    let (_b, mut p) = fixture("tabdir");
    p.cmdline = "cd su".into();
    tab_complete(&mut p);
    assert_eq!(p.cmdline, "cd sub/");
}

#[test]
fn tab_with_multiple_candidates_starts_a_cycle_and_wraps() {
    let (base, mut p) = fixture("tabcycle");
    std::fs::write(base.join("apple.txt"), b"x").unwrap();
    std::fs::write(base.join("avocado.txt"), b"x").unwrap();
    p.cmdline = "cat a".into();
    tab_complete(&mut p);
    let first = p.cmdline.clone();
    assert!(p.complete.is_some(), "multiple matches start a cycle");
    tab_complete(&mut p);
    let second = p.cmdline.clone();
    assert_ne!(first, second, "second Tab advances the cycle");
    tab_complete(&mut p);
    assert_eq!(
        p.cmdline, first,
        "third Tab wraps back to the first candidate"
    );
}

#[test]
fn command_tab_spawns_the_path_scan_exactly_once() {
    // The riskiest wiring: a Command-kind Tab must trigger the background
    // $PATH scan at most once per pane and never block on its result.
    // `bins` is the session-wide cache (see `shared_bins`) and this is the
    // only test in this binary that ever does Command-kind completion, so
    // the cache is guaranteed cold here regardless of test run order —
    // don't add another Command-kind `tab_complete` call anywhere else in
    // this crate's tests without re-checking this assumption.
    let (_b, mut p) = fixture("tabcmdscan");
    p.cmdline = "l".into();
    tab_complete(&mut p);
    assert!(p.bins_scan_started, "first Command Tab starts the scan");
    tab_complete(&mut p);
    assert!(p.bins_scan_started, "second Tab keeps the guard set");
    // Whatever the scan's timing, the call returned without blocking and
    // builtins-only completion stays available meanwhile.
    p.cmdline = "c".into();
    tab_complete(&mut p);
    assert!(p.cmdline.starts_with('c'), "builtins path stays usable");
}

#[test]
fn history_recall_and_ghost_accept_invalidate_an_active_cycle() {
    let (base, mut p) = fixture("cycleinval");
    std::fs::write(base.join("apple.txt"), b"x").unwrap();
    std::fs::write(base.join("avocado.txt"), b"x").unwrap();
    p.history = CmdHistory::from_entries(vec!["cat apple.txt".into()]);
    p.cmdline = "cat a".into();
    tab_complete(&mut p);
    assert!(p.complete.is_some());
    history_prev(&mut p);
    assert!(p.complete.is_none(), "history recall clears the cycle");

    p.cmdline = "cat a".into();
    tab_complete(&mut p);
    assert!(p.complete.is_some());
    accept_ghost(&mut p);
    assert!(p.complete.is_none(), "ghost accept clears the cycle");
}

#[test]
fn escape_during_a_cycle_restores_the_pre_cycle_text() {
    let (base, mut p) = fixture("tabesc");
    std::fs::write(base.join("apple.txt"), b"x").unwrap();
    std::fs::write(base.join("avocado.txt"), b"x").unwrap();
    p.cmdline = "cat a".into();
    tab_complete(&mut p);
    assert!(p.complete.is_some());
    let action = escape_cmdline(&mut p);
    assert!(
        action.is_none(),
        "Esc during a cycle doesn't close the pane"
    );
    assert_eq!(p.cmdline, "cat a");
    assert!(p.complete.is_none());
}

#[test]
fn escape_on_a_typed_bar_clears_it_without_closing() {
    let (_b, mut p) = fixture("tabescclear");
    p.cmdline = "ls".into();
    assert!(escape_cmdline(&mut p).is_none());
    assert!(p.cmdline.is_empty());
}

#[test]
fn escape_on_an_empty_bar_closes_the_pane() {
    let (_b, mut p) = fixture("tabescclose");
    assert!(matches!(escape_cmdline(&mut p), Some(FarAction::Close)));
}

#[test]
fn history_prev_and_next_cycle_through_the_bar() {
    let (_b, mut p) = fixture("histcycle");
    p.history = CmdHistory::from_entries(vec!["ls".into(), "cargo test".into()]);
    p.cmdline = "half".into();
    history_prev(&mut p);
    assert_eq!(p.cmdline, "cargo test");
    history_prev(&mut p);
    assert_eq!(p.cmdline, "ls");
    history_next(&mut p);
    assert_eq!(p.cmdline, "cargo test");
    history_next(&mut p);
    assert_eq!(
        p.cmdline, "half",
        "Down past the newest restores the typed text"
    );
}

#[test]
fn accept_ghost_fills_in_the_matching_history_entry() {
    let (_b, mut p) = fixture("ghostaccept");
    p.history = CmdHistory::from_entries(vec!["cargo build".into()]);
    p.cmdline = "cargo".into();
    accept_ghost(&mut p);
    assert_eq!(p.cmdline, "cargo build");
}

#[test]
fn accept_ghost_is_a_no_op_without_a_match() {
    let (_b, mut p) = fixture("ghostnoop");
    p.history = CmdHistory::from_entries(vec!["cargo build".into()]);
    p.cmdline = "zz".into();
    accept_ghost(&mut p);
    assert_eq!(p.cmdline, "zz");
}

#[test]
fn accept_ghost_during_a_cycle_does_not_insert_an_invisible_ghost() {
    // `render.rs` suppresses the ghost suggestion while a Tab-cycle is
    // active (the cycle's candidate already occupies the line), so
    // Right/End must not silently splice in a history match that was never
    // shown on screen — it should just end the cycle, like Esc does.
    let (base, mut p) = fixture("ghostcycle");
    std::fs::write(base.join("apple.txt"), b"x").unwrap();
    std::fs::write(base.join("avocado.txt"), b"x").unwrap();
    p.history = CmdHistory::from_entries(vec!["cat apple.txt | grep foo".into()]);
    p.cmdline = "cat a".into();
    tab_complete(&mut p);
    assert!(p.complete.is_some(), "multiple matches start a cycle");
    accept_ghost(&mut p);
    assert!(
        !p.cmdline.contains("| grep foo"),
        "Right/End must not accept a ghost that was never rendered: {:?}",
        p.cmdline
    );
    assert!(p.complete.is_none(), "the cycle is cleared, not advanced");
}

/// Point `$HOME` at a fresh tempdir for the duration of `f`, then restore it
/// — callers must hold `super::cmdhist::test_guard()` first. `run_cmdline`
/// persists history via the real `dirs`-based path, so any test exercising
/// it needs this isolation (mirrors `cmdhist.rs`'s own test helper — kept
/// separate since it's a different file/module).
fn with_tmp_home<T>(f: impl FnOnce() -> T) -> T {
    let dir = tempfile::tempdir().unwrap();
    let prev = std::env::var_os("HOME");
    std::env::set_var("HOME", dir.path());
    let out = f();
    match prev {
        Some(p) => std::env::set_var("HOME", p),
        None => std::env::remove_var("HOME"),
    }
    out
}

#[test]
fn run_cmdline_pushes_the_command_into_history() {
    let _g = super::cmdhist::test_guard();
    with_tmp_home(|| {
        let (base, mut p) = fixture("histpush");
        p.right.cwd = base.join("sub");
        p.active = Side::Right;
        p.cmdline = "touch made-here".into();
        run_cmdline(&mut p);
        assert_eq!(p.history.prev(""), Some("touch made-here"));
    });
}

#[test]
fn cd_is_pushed_into_history_too() {
    let _g = super::cmdhist::test_guard();
    with_tmp_home(|| {
        let (_b, mut p) = fixture("histpushcd");
        p.cmdline = "cd sub".into();
        run_cmdline(&mut p);
        assert_eq!(p.history.prev(""), Some("cd sub"));
    });
}

#[test]
fn new_pane_starts_with_no_ask() {
    let (_b, p) = fixture("noask");
    assert!(p.ask.is_none());
}

#[test]
fn absorb_ask_result_lands_a_suggestion_and_replaces_the_bar() {
    let (_b, mut p) = fixture("askland");
    p.cmdline = "! list files".into();
    let msg = p.absorb_ask_result(Ok("ls -la".into()));
    assert_eq!(p.cmdline, "ls -la");
    assert!(matches!(&p.ask, Some(AskState::Suggested { original }) if original == "! list files"));
    assert!(msg.contains("Enter run"));
}

#[test]
fn absorb_ask_result_treats_a_blank_suggestion_as_no_command() {
    let (_b, mut p) = fixture("askblank");
    p.cmdline = "! list files".into();
    let msg = p.absorb_ask_result(Ok("   ".into()));
    assert_eq!(
        p.cmdline, "! list files",
        "the ! text is kept on an empty reply"
    );
    assert!(p.ask.is_none());
    assert!(msg.contains("no command"));
}

#[test]
fn absorb_ask_result_surfaces_a_provider_error_and_keeps_the_bang_text() {
    let (_b, mut p) = fixture("askerr");
    p.cmdline = "! list files".into();
    let msg = p.absorb_ask_result(Err("no AI provider".into()));
    assert_eq!(p.cmdline, "! list files");
    assert!(p.ask.is_none());
    assert!(msg.contains("no AI provider"));
}

#[test]
fn poll_ask_returns_none_while_still_thinking() {
    let (_b, mut p) = fixture("askthinking");
    let (_tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
    p.ask = Some(AskState::Thinking {
        started: std::time::Instant::now(),
        rx,
    });
    assert!(p.poll_ask().is_none());
    assert!(p.ask.is_some(), "still thinking — ask state untouched");
}

#[test]
fn poll_ask_drains_a_landed_result_via_absorb() {
    let (_b, mut p) = fixture("askdrain");
    let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
    tx.send(Ok("ls -la".into())).unwrap();
    p.cmdline = "! list files".into();
    p.ask = Some(AskState::Thinking {
        started: std::time::Instant::now(),
        rx,
    });
    let msg = p.poll_ask();
    assert_eq!(p.cmdline, "ls -la");
    assert!(msg.unwrap().contains("Enter run"));
}

#[test]
fn poll_ask_handles_a_dead_worker_thread() {
    let (_b, mut p) = fixture("askdead");
    let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
    drop(tx); // disconnect without sending — worker panicked/died
    p.cmdline = "! list files".into();
    p.ask = Some(AskState::Thinking {
        started: std::time::Instant::now(),
        rx,
    });
    let msg = p.poll_ask();
    assert_eq!(p.cmdline, "! list files");
    assert!(p.ask.is_none());
    assert!(msg.unwrap().contains("worker died"));
}

#[test]
fn submit_ask_starts_thinking_and_keeps_the_bang_text() {
    // Guard + mock: without these the spawned worker would dial a REAL
    // provider on machines whose shell env carries API keys.
    let _g = super::ask::test_guard();
    std::env::set_var("CREW_BROKER_MOCK_REPLY", "ls -la");
    let (_b, mut p) = fixture("bangenter");
    p.cmdline = "! list files".into();
    let action = submit_ask(&mut p, "list files");
    std::env::remove_var("CREW_BROKER_MOCK_REPLY");
    assert!(matches!(action, FarAction::Status(ref s) if s.contains("asking ai")));
    assert!(matches!(p.ask, Some(AskState::Thinking { .. })));
    assert_eq!(p.cmdline, "! list files", "the ! text stays while thinking");
}

#[test]
fn submit_ask_nags_on_a_blank_description() {
    let (_b, mut p) = fixture("bangblank");
    let action = submit_ask(&mut p, "");
    assert!(matches!(action, FarAction::Status(ref s) if s.contains("description")));
    assert!(p.ask.is_none());
}

#[test]
fn submit_ask_refuses_a_second_ask_while_one_is_in_flight() {
    let (_b, mut p) = fixture("bangbusy");
    let (_tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
    p.ask = Some(AskState::Thinking {
        started: std::time::Instant::now(),
        rx,
    });
    let action = submit_ask(&mut p, "another one");
    assert!(matches!(action, FarAction::Status(ref s) if s.contains("wait")));
}

#[test]
fn escape_on_a_suggestion_restores_the_original_bang_text() {
    let (_b, mut p) = fixture("bangesc");
    p.cmdline = "ls -la".into();
    p.ask = Some(AskState::Suggested {
        original: "! list files".into(),
    });
    assert!(escape_cmdline(&mut p).is_none());
    assert_eq!(p.cmdline, "! list files");
    assert!(p.ask.is_none());
}

#[test]
fn escape_while_thinking_cancels_the_ask_and_clears_the_bar() {
    let (_b, mut p) = fixture("bangescthink");
    let (_tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
    p.cmdline = "! list files".into();
    p.ask = Some(AskState::Thinking {
        started: std::time::Instant::now(),
        rx,
    });
    assert!(escape_cmdline(&mut p).is_none());
    assert!(p.ask.is_none());
    assert!(
        p.cmdline.is_empty(),
        "Esc's normal non-empty-bar clear still applies"
    );
}

#[test]
fn run_cmdline_after_accepting_a_suggestion_clears_the_ask_state() {
    let _g = super::ask::test_guard();
    with_tmp_home(|| {
        let (base, mut p) = fixture("bangaccept");
        p.right.cwd = base.join("sub");
        p.active = Side::Right;
        p.cmdline = "touch made-here".into();
        p.ask = Some(AskState::Suggested {
            original: "! make a file".into(),
        });
        run_cmdline(&mut p);
        assert!(p.ask.is_none());
        assert_eq!(
            p.history.prev(""),
            Some("touch made-here"),
            "history records the final command, not the ! ask"
        );
    });
}

#[test]
fn bang_ask_end_to_end_with_the_mock_provider() {
    let _g = super::ask::test_guard();
    std::env::set_var("CREW_BROKER_MOCK_REPLY", "ls -la");
    let (_b, mut p) = fixture("bange2e");
    p.cmdline = "! list files".into();
    submit_ask(&mut p, "list files");
    let mut landed = None;
    for _ in 0..300 {
        if let Some(msg) = p.poll_ask() {
            landed = Some(msg);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    std::env::remove_var("CREW_BROKER_MOCK_REPLY");
    assert!(
        landed.unwrap().contains("Enter run"),
        "the hint reaches the caller"
    );
    assert_eq!(p.cmdline, "ls -la");
    assert!(matches!(&p.ask, Some(AskState::Suggested { original }) if original == "! list files"));
}

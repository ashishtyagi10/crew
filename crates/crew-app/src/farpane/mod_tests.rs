use super::keys::{activate, ascend, move_sel};
use super::run::run_cmdline;
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
    let (_b, p) = fixture("newstate");
    assert!(p.complete.is_none());
    assert!(p.bins.get().is_none());
    assert!(!p.bins_scan_started);
}

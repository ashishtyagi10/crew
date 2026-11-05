use super::{matches_in, view_for};
use crate::app::CrewApp;
use crate::pane::{spawn_pane, PaneContent};
use crew_term::{GridSize, TermModel};

#[test]
fn a_hit_in_view_leaves_the_view_alone() {
    // 24 rows at offset 100 show depths 100..=123.
    assert_eq!(view_for(100, 100, 24, 9000), 100);
    assert_eq!(view_for(123, 100, 24, 9000), 100);
}

#[test]
fn a_hit_out_of_view_lands_mid_view_within_the_scrollback() {
    // Depth 5000 at row 12 of 24: offset 5000 - 11 puts it there.
    assert_eq!(view_for(5000, 0, 24, 9000), 4989);
    // Near the top of the history the view stops at the top.
    assert_eq!(view_for(9020, 0, 24, 9000), 9000);
    // Below the view (an older view scrolled up past it): back down.
    assert_eq!(view_for(30, 500, 24, 9000), 19);
}

#[test]
fn matches_fold_like_the_screen() {
    assert_eq!(matches_in("Error error ERROR", "error", true), 3);
    assert_eq!(matches_in("Error error ERROR", "Error", false), 1);
    assert_eq!(matches_in("anything", "", true), 0);
}

/// A real pane (fed directly, no shell output needed): `/find` reaches a
/// line 7,000 back in one call, shows it mid-view, and a repeat walks on.
#[test]
fn find_reaches_the_whole_scrollback_and_repeats_walk_older() {
    let grid = GridSize { cols: 80, rows: 24 };
    let mut app = CrewApp::default();
    app.panes
        .push(spawn_pane("sh", "sh", grid, Some(&std::env::temp_dir())).unwrap());
    let mut out = "before\r\n".repeat(50);
    out.push_str("needlezq oldest\r\nneedlezq older\r\n");
    for i in 0..7000 {
        out.push_str(&format!("line {i} of filler output\r\n"));
    }
    let offset = |app: &CrewApp| match &app.panes[0].content {
        PaneContent::Terminal(t) => t.pty.display_offset(),
        _ => unreachable!(),
    };
    if let PaneContent::Terminal(t) = &mut app.panes[0].content {
        t.pty.feed(b"\x1b[H\x1b[2J\x1b[3J");
        t.pty.feed(out.as_bytes());
        t.pty.scroll_to_bottom();
    }
    app.find_in_terminal("needlezq");
    let status = app
        .status
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default();
    assert!(status.contains("in view"), "found: {status}");
    // The newer needle is 7,001 lines above the cursor's row (7,000 filler
    // lines), and sits at row 12 of 24: offset 7001 - 11. The shell's own
    // output is never read here (no `try_read`), so nothing shifts it.
    let first = offset(&app);
    assert_eq!(first, 6990);
    app.find_in_terminal("needlezq");
    assert!(offset(&app) >= first, "a repeat never walks back down");
    app.find_in_terminal("needlezq");
    let status = app
        .status
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default();
    assert!(
        status.starts_with("no older match"),
        "then runs out: {status}"
    );
}

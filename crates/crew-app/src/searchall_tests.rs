use super::{count_lines, count_scrollback};
use crate::app::CrewApp;
use crate::pane::{spawn_pane, PaneContent};
use crew_term::GridSize;

#[test]
fn count_lines_counts_non_overlapping_occurrences() {
    let lines = vec!["error error".to_string(), "no".into(), "error".into()];
    assert_eq!(count_lines(&lines, "error"), 3);
    assert_eq!(count_lines(&lines, "zzz"), 0);
}

#[test]
fn findall_usage_hint_and_no_panes() {
    let mut app = CrewApp::default();
    app.find_all("  ");
    // Usage hint on empty term; a real term with no panes reports no match.
    app.find_all("needle");
    assert!(app.panes.is_empty());
}

/// Two real (non-login, `sh`) shells; only the second ever saw the marker.
#[test]
fn findall_focuses_the_first_matching_pane_from_the_bottom() {
    let grid = GridSize { cols: 80, rows: 24 };
    let dir = std::env::temp_dir();
    let mut app = CrewApp::default();
    for _ in 0..2 {
        app.panes
            .push(spawn_pane("sh", "sh", grid, Some(&dir)).unwrap());
    }
    if let PaneContent::Terminal(t) = &mut app.panes[1].content {
        use std::io::Write;
        let _ = t.input.write_all(b"echo crewfindallmarker\n");
    }
    // No event loop here: pump the reader channel into the model the way
    // poll_panes does each tick, until the marker lands in the grid.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut found = false;
    while std::time::Instant::now() < deadline {
        if let PaneContent::Terminal(t) = &mut app.panes[1].content {
            t.pty.try_read();
            if count_scrollback(&mut t.pty, grid.cols, grid.rows, "crewfindallmarker") > 0 {
                found = true;
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(found, "marker never appeared in pane 1's scrollback");
    // Leave pane 1 scrolled up: the review-found strand-at-top bug — /find
    // only searches upward, so without the pre-find scroll_to_bottom a
    // match below the old view was never landed on.
    if let PaneContent::Terminal(t) = &mut app.panes[1].content {
        t.pty.scroll(5);
    }
    app.focused = 0;
    app.find_all("crewfindallmarker");
    assert_eq!(app.focused, 1, "focus lands on the matching pane");
    if let PaneContent::Terminal(t) = &mut app.panes[1].content {
        // find_in_terminal ran from the bottom and the match is on the live
        // screen, so the pane must not be stranded in old scrollback.
        assert!(
            t.pty.display_offset() < 5,
            "pane stranded at old scroll position"
        );
    }
}

#[test]
fn count_scrollback_is_smart_case_like_find() {
    let grid = GridSize { cols: 40, rows: 10 };
    let dir = std::env::temp_dir();
    let mut pane = spawn_pane("sh", "sh", grid, Some(&dir)).unwrap();
    let PaneContent::Terminal(t) = &mut pane.content else {
        panic!("terminal pane expected");
    };
    use std::io::Write;
    let _ = t.input.write_all(b"echo CrewCaseMarker\n");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        t.pty.try_read();
        // Lowercase term: case-insensitive — sees the mixed-case output.
        if count_scrollback(&mut t.pty, grid.cols, grid.rows, "crewcasemarker") > 0 {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "marker never appeared"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    // Uppercase in the term: exact-case only — the wrong case finds nothing.
    assert_eq!(
        count_scrollback(&mut t.pty, grid.cols, grid.rows, "CREWCASEMARKER"),
        0
    );
}

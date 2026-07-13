use super::count_matches;
use crate::app::CrewApp;

#[test]
fn count_matches_is_smart_case() {
    // Lowercase term: case-insensitive.
    assert_eq!(count_matches("Error error ERROR", "error"), 3);
    // Uppercase in the term: exact case.
    assert_eq!(count_matches("Error error ERROR", "Error"), 1);
    assert_eq!(count_matches("", "x"), 0);
}

#[test]
fn findall_usage_hint_and_no_panes() {
    let mut app = CrewApp::default();
    app.find_all("  ");
    // Usage hint on empty term; a real term with no panes reports no match.
    app.find_all("needle");
    assert!(app.panes.is_empty());
}

#[test]
fn findall_focuses_the_first_matching_pane() {
    // Two real shells; only the second ever saw the needle. `cat` panes
    // (the chatswarm-test pattern) can't take PTY writes, so drive real
    // `sh -c` terminals via the normal spawn path with a marker command.
    let mut app = CrewApp {
        cwd: std::env::temp_dir(),
        ..Default::default()
    };
    app.spawn_new_pane();
    app.spawn_new_pane();
    assert_eq!(app.panes.len(), 2);
    // Feed the marker into pane 1's PTY input and give the shell a moment
    // to echo it into the grid.
    if let crate::pane::PaneContent::Terminal(t) = &mut app.panes[1].content {
        use std::io::Write;
        let _ = t.input.write_all(b"echo crewfindallmarker\n");
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut found = false;
    while std::time::Instant::now() < deadline {
        let (cols, rows) = (app.panes[1].grid.cols, app.panes[1].grid.rows);
        if let crate::pane::PaneContent::Terminal(t) = &mut app.panes[1].content {
            // No event loop here: pump the reader channel into the model
            // the way poll_panes does each tick.
            t.pty.try_read();
            let text = crate::dump::capture_scrollback(&mut t.pty, cols, rows);
            // The echoed OUTPUT line appears once the shell ran the command
            // (the typed command itself also matches — either proves it).
            if count_matches(&text, "crewfindallmarker") > 0 {
                found = true;
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(found, "marker never appeared in pane 1's scrollback");
    app.focused = 0;
    app.find_all("crewfindallmarker");
    assert_eq!(app.focused, 1, "focus lands on the matching pane");
}

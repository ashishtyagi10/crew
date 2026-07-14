use super::*;
use std::io::Write;
use std::time::{Duration, Instant};

#[test]
fn echo_roundtrips_through_pty() {
    let mut term = PtyTerm::spawn(GridSize { cols: 40, rows: 10 }, "sh").unwrap();
    let mut w = term.writer();
    // Echo a unique token, then read until it shows up on the grid.
    w.write_all(b"printf CREWOK\n").unwrap();
    w.flush().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut found = false;
    while Instant::now() < deadline {
        term.try_read();
        let line: String = {
            let mut cs: Vec<_> = term.cells(true);
            cs.sort_by_key(|c| (c.row, c.col));
            cs.iter().map(|c| c.c).collect()
        };
        if line.contains("CREWOK") {
            found = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(found, "expected CREWOK to appear on the terminal grid");
}

#[test]
fn try_read_caps_bytes_per_tick_under_flood() {
    // A program that floods stdout: a single tick must not drain the whole
    // backlog, or it would block the event loop (and every other pane).
    let mut term = PtyTerm::spawn(GridSize { cols: 80, rows: 24 }, "sh").unwrap();
    let mut w = term.writer();
    w.write_all(b"yes crew-flood-line\n").unwrap();
    w.flush().unwrap();
    // Let the reader thread buffer well past one tick's budget.
    std::thread::sleep(Duration::from_millis(250));

    // The budget is checked between chunks, so the final 8 KiB reader chunk
    // can overshoot slightly — the point is the drain is *bounded* to roughly
    // the budget instead of consuming the whole flood (which would hang).
    let n = term.try_read();
    assert!(
        n <= READ_BUDGET + 8192,
        "one tick drained {n} bytes, far over the {READ_BUDGET}-byte budget"
    );
    assert!(
        term.has_pending(),
        "expected a backlog to remain after a budget-capped read"
    );

    // Stop `yes` so the child doesn't keep spinning after the test.
    let _ = w.write_all(&[0x03]); // Ctrl-C to the foreground process group
    let _ = w.flush();
}

#[test]
fn capture_records_raw_output_only_while_on() {
    let mut term = PtyTerm::spawn(GridSize { cols: 40, rows: 10 }, "sh").unwrap();
    assert_eq!(term.take_capture(), "", "nothing captured before start");
    term.start_capture();
    let mut w = term.writer();
    w.write_all(b"printf 'MARK-42'\n").unwrap();
    w.flush().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut got = String::new();
    while Instant::now() < deadline {
        term.try_read();
        got.push_str(&term.take_capture());
        if got.contains("MARK-42") {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        got.contains("MARK-42"),
        "capture records raw output: {got:?}"
    );

    // Once stopped, subsequent output is not captured.
    term.stop_capture();
    w.write_all(b"printf 'AFTER'\n").unwrap();
    w.flush().unwrap();
    std::thread::sleep(Duration::from_millis(200));
    term.try_read();
    assert!(
        !term.take_capture().contains("AFTER"),
        "stopped capture records nothing"
    );
}

fn pats(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| s.to_string()).collect()
}

#[test]
fn scan_matches_a_completed_line() {
    let mut tail = String::new();
    let hits = scan(&mut tail, b"Build succeeded\n", &pats(&["build succeeded"]));
    assert_eq!(hits, vec!["build succeeded".to_string()]);
}

#[test]
fn scan_is_case_insensitive() {
    let mut tail = String::new();
    let hits = scan(&mut tail, b"ERROR: boom\n", &pats(&["error"]));
    assert_eq!(hits, vec!["error".to_string()]);
}

#[test]
fn scan_ignores_ansi_color_codes() {
    let mut tail = String::new();
    // Red "error" wrapped in SGR codes, plus an OSC title set.
    let chunk = b"\x1b]0;title\x07\x1b[31merror\x1b[0m here\n";
    let hits = scan(&mut tail, chunk, &pats(&["error"]));
    assert_eq!(hits, vec!["error".to_string()]);
}

#[test]
fn scan_matches_across_a_chunk_boundary() {
    let mut tail = String::new();
    // The pattern is split across two reads; no newline yet → no match.
    let first = scan(&mut tail, b"Build suc", &pats(&["build succeeded"]));
    assert!(first.is_empty());
    // The newline completes the line and the carried tail makes it match.
    let second = scan(&mut tail, b"ceeded\n", &pats(&["build succeeded"]));
    assert_eq!(second, vec!["build succeeded".to_string()]);
}

#[test]
fn scan_does_not_rematch_an_already_consumed_line() {
    let mut tail = String::new();
    let first = scan(&mut tail, b"error here\n", &pats(&["error"]));
    assert_eq!(first, vec!["error".to_string()]);
    // A later read with no new match must not re-report the old line.
    let second = scan(&mut tail, b"all good\n", &pats(&["error"]));
    assert!(second.is_empty());
}

#[test]
fn scan_empty_patterns_is_a_noop() {
    let mut tail = String::new();
    let hits = scan(&mut tail, b"anything at all\n", &[]);
    assert!(hits.is_empty());
    assert!(
        tail.is_empty(),
        "no work and no buffering when nothing is watched"
    );
}

#[test]
fn scan_keeps_the_tail_bounded_under_a_newline_free_flood() {
    let mut tail = String::new();
    let pat = pats(&["needle"]);
    // 100 KiB with no newline must not grow the carry without bound.
    for _ in 0..1000 {
        scan(&mut tail, &[b'x'; 100], &pat);
    }
    assert!(tail.len() <= 4096, "carry grew to {} bytes", tail.len());
}

#[test]
fn strip_ansi_removes_csi_and_osc() {
    assert_eq!(strip_ansi("\x1b[1;31mhi\x1b[0m"), "hi");
    assert_eq!(strip_ansi("\x1b]0;my title\x07done"), "done");
    assert_eq!(strip_ansi("plain"), "plain");
}

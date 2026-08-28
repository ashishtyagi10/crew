use super::*;

fn info(s: &str) -> LogEntry {
    LogEntry {
        level: LogLevel::Info,
        text: s.to_string(),
    }
}

#[test]
fn log_section_has_rule_and_newest_last() {
    let entries = [info("first"), info("second")];
    let cells = log_cells(&entries, 24, 5, 0);
    // LOG rule + legend on row 0
    assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
    assert!(cells.iter().any(|c| c.c == 'L' && c.row == 0));
    // oldest entry on row 1, newest on row 2 (bottom)
    assert!(cells.iter().any(|c| c.c == 'f' && c.row == 1));
    assert!(cells.iter().any(|c| c.c == 's' && c.row == 2));
}

#[test]
fn log_keeps_only_the_most_recent_lines() {
    let entries: Vec<LogEntry> = (0..10).map(|i| info(&format!("line{i}"))).collect();
    let cells = log_cells(&entries, 24, 3, 0);
    // only the last 3 entries are drawn (rows 1..=3); nothing on row 4
    assert!(!cells.iter().any(|c| c.row == 4));
    // the oldest shown is line7 (10 entries, last 3) — its '7' is on row 1
    assert!(cells.iter().any(|c| c.c == '7' && c.row == 1));
}

#[test]
fn error_entries_render_in_the_bell_color() {
    // Two reads of the process-global theme (inside `log_cells` and again
    // below): serialised, or a theme-mutating test flips it in between and
    // the two disagree under the parallel runner.
    let _g = crate::app::theme_test_guard();
    let entries = [
        info("fine"),
        LogEntry {
            level: LogLevel::Error,
            text: "broke".to_string(),
        },
    ];
    let cells = log_cells(&entries, 24, 5, 0);
    let t = crew_theme::theme();
    // The info line (row 1) is muted; the error line (row 2) is bell.
    let fg_at = |row, ch| cells.iter().find(|c| c.row == row && c.c == ch).unwrap().fg;
    assert_eq!(fg_at(1, 'f'), t.text_muted);
    assert_eq!(fg_at(2, 'b'), t.bell);
}

/// The five-row window is a *window*: scrolling it back must show older
/// entries, and must stop on the oldest rather than scrolling into nothing.
#[test]
fn scrolling_back_shows_older_entries_and_stops_at_the_oldest() {
    let entries: Vec<LogEntry> = (0..10).map(|i| info(&format!("line{i}"))).collect();
    let oldest_shown = |back: usize| -> Option<char> {
        let cells = log_cells(&entries, 24, 3, back);
        let c = cells.iter().find(|c| c.row == 1 && c.c.is_ascii_digit())?;
        Some(c.c)
    };
    // At the live edge the window ends on line9, so it starts on line7.
    assert_eq!(oldest_shown(0), Some('7'), "following the newest");
    assert_eq!(oldest_shown(2), Some('5'), "two lines back");
    assert_eq!(oldest_shown(7), Some('0'), "the oldest entry there is");
    assert_eq!(oldest_shown(99), Some('0'), "and it stops there");
}

/// A scrolled-back log says so on its rule — one that silently stopped
/// following would look like a log that stopped.
#[test]
fn a_scrolled_back_log_marks_its_rule() {
    let entries: Vec<LogEntry> = (0..10).map(|i| info(&format!("line{i}"))).collect();
    let has_mark = |back| {
        log_cells(&entries, 24, 3, back)
            .iter()
            .any(|c| c.row == 0 && c.c == '\u{21e1}')
    };
    assert!(!has_mark(0), "a live tail is unmarked");
    assert!(has_mark(3), "a scrolled one is marked");
}

#[test]
fn the_window_never_runs_off_either_end() {
    for n in 0..12usize {
        for back in 0..15usize {
            let (start, shown) = window(n.max(1), 5, back);
            assert!(start + shown <= n.max(1), "n={n} back={back}");
        }
    }
}

#[test]
fn empty_or_narrow_renders_nothing() {
    assert!(log_cells(&[], 24, 5, 0).is_empty());
    assert!(log_cells(&[info("x")], 24, 0, 0).is_empty());
    assert!(log_cells(&[info("x")], 3, 5, 0).is_empty());
}

/// A line too long for the nav ends in `…`, not mid-word. The nav is narrow
/// enough that most real lines overflow, so the clip is the common case and
/// "updated to cr" is what it used to look like.
#[test]
fn an_overflowing_line_ends_in_an_ellipsis() {
    let _g = crate::app::theme_test_guard();
    let e = [info("23:20 updated to crew v0.19.38")];
    let row: String = {
        let mut v: Vec<_> = log_cells(&e, 24, 5, 0)
            .into_iter()
            .filter(|c| c.row == 1)
            .collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert!(row.starts_with("23:20 "), "the stamp survives: {row:?}");
    assert!(row.ends_with('\u{2026}'), "the message is clipped: {row:?}");
    assert!(
        !row.contains("v0.19.38"),
        "…because it did not fit: {row:?}"
    );
}

/// The stamp is furniture — same six columns on every line — so it is dimmed
/// and the message keeps the ink. A line with no stamp gives all of it away.
#[test]
fn the_stamp_is_dimmed_and_the_message_is_not() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    let cells = log_cells(&[info("23:20 ok")], 24, 5, 0);
    let row1 = |col: u16| cells.iter().find(|c| c.row == 1 && c.col == col);
    assert_eq!(row1(2).map(|c| c.fg), Some(t.dim), "stamp is dim");
    assert_eq!(row1(8).map(|c| c.fg), Some(t.text_muted), "message is not");

    let plain = log_cells(&[info("ok")], 24, 5, 0);
    assert!(
        plain.iter().filter(|c| c.row == 1).all(|c| c.fg != t.dim),
        "an unstamped line gives no columns to a stamp"
    );
}

/// A run of lines from the same minute prints its stamp once — a stack of
/// identical `23:12`s is furniture repeating itself. The messages stay in
/// their column either way, so the lines still align.
#[test]
fn a_repeated_stamp_is_printed_once() {
    let _g = crate::app::theme_test_guard();
    let e = [
        info("23:12 one"),
        info("23:12 two"),
        info("23:13 three"),
        info("23:13 four"),
    ];
    let cells = log_cells(&e, 24, 5, 0);
    let row = |r: u16| -> String {
        let mut v: Vec<_> = cells.iter().filter(|c| c.row == r).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert_eq!(row(1), "23:12 one");
    assert_eq!(row(2), "      two", "the repeat is blanked, not shifted");
    assert_eq!(row(3), "23:13 three", "a new minute prints again");
    assert_eq!(row(4), "      four");
}

//! The row fits the pane it is opened in. Split from `toolsview_tests`; the
//! picture of it is `toolshot_tests`.
use super::tests::{rec, NOW};
use super::*;

// ---------------------------------------------------------------------------
// The row fits the pane it is opened in
// ---------------------------------------------------------------------------

/// A viewer opened as one tile of a grid is nearer 47 columns than 80.
/// The row was 80 — a clock, a padded tier, a padded outcome, a padded tool
/// and a requester — so every line wrapped in the place this is most often
/// read.
#[test]
fn no_row_is_wider_than_a_tiled_viewer() {
    // The widest relative time is `23h ago` — a day becomes `1d ago`.
    let mut worst = rec(
        NOW - (23 * 3600 + 59 * 60 + 59) * 1000,
        "some-long-server:some_long_tool_name",
        "irreversible",
        "ask",
        "timed_out",
    );
    worst.requester = "channel:a-fairly-long-address".into();
    worst.note = "nobody answered in five minutes".into();
    for line in listing(&[worst], 0, "", NOW).lines() {
        assert!(
            line.chars().count() <= 47,
            "{} cols: {line:?}",
            line.chars().count()
        );
    }
}

/// The common call — a person at this keyboard, running something that worked
/// — earns one line and no commentary. A listing that spelled `ran` and `pane`
/// on every such row was spending its width saying "normal".
#[test]
fn an_ordinary_call_takes_one_quiet_line() {
    let out = listing(&[rec(1_000, "sys:run", "read", "allow", "ran")], 0, "", NOW);
    let rows: Vec<&str> = out.lines().filter(|l| l.contains("sys:run")).collect();
    assert_eq!(rows.len(), 1, "{out}");
    assert!(!rows[0].contains("ran"), "the tick already said it: {out}");
    assert!(!rows[0].contains("pane"), "who else would it be: {out}");
}

/// …and an unusual one says what was unusual, all of it, on the indented
/// detail lines under its row — wrapped to the tile rather than clipped.
#[test]
fn an_unusual_call_explains_itself_underneath() {
    let mut r = rec(1_000, "gmail:send", "irreversible", "ask", "timed_out");
    r.requester = "channel:telegram".into();
    r.note = "nobody answered".into();
    let out = listing(&[r], 0, "", NOW);
    let row = out
        .lines()
        .position(|l| l.contains("gmail:send"))
        .expect("the row");
    let details: Vec<&str> = out
        .lines()
        .skip(row + 1)
        .take_while(|l| l.starts_with("             "))
        .collect();
    assert!(!details.is_empty(), "indented under its row: {out}");
    let detail = details.join(" ");
    assert!(detail.contains("timed_out"), "{detail}");
    assert!(detail.contains("channel:telegram"), "{detail}");
    assert!(detail.contains("nobody answered"), "{detail}");
}

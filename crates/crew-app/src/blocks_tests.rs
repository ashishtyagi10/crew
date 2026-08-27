use super::*;

fn spans() -> Spans {
    let mut s = Spans::default();
    s.started("cargo build".into(), 0, 0);
    s.finished(Some(0), 40, 12_000);
    s.started("cargo test".into(), 40, 20_000);
    s.finished(Some(101), 90, 144_000);
    s.started("ls".into(), 90, 150_000);
    s.close_at(92, 150_400);
    s
}

/// Newest first, and numbered the way `/out`'s argument is numbered — the
/// point of pairing the two.
#[test]
fn the_rows_are_numbered_the_way_out_counts_back() {
    let text = listing(&spans(), "shell", 150_400);
    let rows: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("  0") || l.starts_with("  1") || l.starts_with("  2"))
        .collect();
    assert_eq!(rows.len(), 3);
    assert!(rows[0].contains("ls"), "{rows:?}");
    assert!(rows[1].contains("cargo test"), "{rows:?}");
    assert!(rows[2].contains("cargo build"), "{rows:?}");
    assert!(text.contains("/out <n>"), "the pairing is said out loud");
}

/// A block with no reported status is `·`, not `✓`. Crew only knows how a
/// command ended when the shell says so, and drawing "no answer" as success
/// would be inventing one.
#[test]
fn no_reported_status_is_not_a_tick() {
    let text = listing(&spans(), "shell", 150_400);
    let row = text.lines().find(|l| l.contains("ls")).unwrap();
    assert!(row.contains('\u{b7}'), "{row:?}");
    assert!(!row.contains('\u{2713}'), "{row:?}");
    let ok = text.lines().find(|l| l.contains("cargo build")).unwrap();
    assert!(ok.contains('\u{2713}'), "a reported zero IS a tick: {ok:?}");
    let bad = text.lines().find(|l| l.contains("cargo test")).unwrap();
    assert!(bad.contains("\u{2717} 101"), "{bad:?}");
}

/// A block still running says so, and reports how long it has been at it
/// rather than nothing at all.
#[test]
fn a_running_block_says_so_and_counts_up() {
    let mut s = Spans::default();
    s.started("cargo build".into(), 0, 1_000);
    let text = listing(&s, "shell", 95_000);
    let row = text.lines().find(|l| l.contains("cargo build")).unwrap();
    assert!(row.contains("running"), "{row:?}");
    assert!(row.contains("1m34"), "{row:?}");
}

#[test]
fn the_elapsed_field_is_as_long_as_it_needs_and_no_longer() {
    assert_eq!(elapsed(400), "400ms");
    assert_eq!(elapsed(999), "999ms");
    assert_eq!(elapsed(1_000), "1s");
    assert_eq!(elapsed(59_999), "59s");
    assert_eq!(elapsed(60_000), "1m00");
    assert_eq!(elapsed(3_601_000), "60m01");
    // Nothing ever outgrows the column the names are aligned against.
    for ms in [0u64, 1, 999, 60_000, 3_600_000, 86_400_000] {
        assert!(
            elapsed(ms).chars().count() <= TIME_W,
            "{ms} → {}",
            elapsed(ms)
        );
    }
}

#[test]
fn a_pane_that_has_run_nothing_says_so() {
    let text = listing(&Spans::default(), "shell", 0);
    assert!(text.contains("nothing has run in this pane yet"));
    assert!(!text.contains("/out <n>"), "no numbers to pair with");
}

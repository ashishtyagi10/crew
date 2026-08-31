use super::*;

fn rec(ts_ms: u64, tool: &str, tier: &str, decision: &str, outcome: &str) -> Record {
    Record {
        ts_ms,
        tool: tool.into(),
        tier: tier.into(),
        requester: "pane".into(),
        decision: decision.into(),
        outcome: outcome.into(),
        note: String::new(),
    }
}

#[test]
fn an_empty_ledger_says_what_will_land_there() {
    let out = listing(&[], 0, "");
    assert!(out.contains("Nothing yet"), "{out}");
    // No headings for columns that have no rows.
    assert!(!out.contains("\u{2713}"), "{out}");
}

#[test]
fn the_newest_call_is_first() {
    let out = listing(
        &[
            rec(1_000, "sys:list_dir", "read", "allow", "ran"),
            rec(2_000, "sys:run", "irreversible", "allow", "ran"),
        ],
        0,
        "",
    );
    let run = out.find("sys:run").expect("run listed");
    let ls = out.find("sys:list_dir").expect("list_dir listed");
    assert!(run < ls, "newest first:\n{out}");
}

/// A record whose call never returned is `·`, not `✓`. The gate writes the
/// decision when it makes it and the outcome when the call ends; a crash
/// between the two leaves a real row with nothing after it, and drawing that
/// as success invents an answer.
#[test]
fn an_unfinished_call_is_not_reported_as_success() {
    let out = listing(&[rec(1_000, "sys:run", "irreversible", "allow", "")], 0, "");
    assert!(out.contains('\u{b7}'), "{out}");
    assert!(!out.contains('\u{2713}'), "{out}");
}

#[test]
fn a_denial_reads_as_one() {
    let out = listing(
        &[rec(1_000, "gmail:send", "irreversible", "deny", "")],
        0,
        "",
    );
    assert!(out.contains("\u{2717} denied"), "{out}");
}

#[test]
fn a_note_is_shown_under_its_row() {
    let mut r = rec(1_000, "sys:run", "irreversible", "allow", "failed");
    r.note = "exit 127: command not found".into();
    let out = listing(&[r], 0, "");
    assert!(out.contains("exit 127"), "{out}");
}

/// A history with a hole in it that SAYS SO is worth more than one that
/// quietly shows fewer rows.
#[test]
fn unreadable_lines_are_reported_not_swallowed() {
    let out = listing(&[rec(1, "sys:run", "read", "allow", "ran")], 3, "");
    assert!(out.contains("3 unreadable"), "{out}");
}

/// The ledger is append-only and unbounded — the record of a machine, not of
/// a session — so the view takes the tail and says what it left out.
#[test]
fn a_long_ledger_is_bounded_and_says_how_much_it_hid() {
    let many: Vec<Record> = (0..MAX_ROWS as u64 + 25)
        .map(|i| rec(i, "sys:run", "read", "allow", "ran"))
        .collect();
    let out = listing(&many, 0, "");
    assert_eq!(
        out.lines().filter(|l| l.contains("sys:run")).count(),
        MAX_ROWS
    );
    assert!(
        out.contains("25 older call(s) not shown"),
        "tail note missing"
    );
}

#[test]
fn the_clock_reads_as_a_time_of_day() {
    // 14:03:22 UTC on any day.
    let ts = (14 * 3600 + 3 * 60 + 22) * 1000;
    assert_eq!(clock(ts), "14:03:22");
    assert_eq!(clock(0), "00:00:00");
}

// ---------------------------------------------------------------------------
// Narrowing a thousand rows
// ---------------------------------------------------------------------------

fn sample() -> Vec<Record> {
    let mut denied = rec(3_000, "gmail:send", "irreversible", "deny", "");
    denied.note = "no human at the keyboard".into();
    vec![
        rec(1_000, "sys:list_dir", "read", "allow", "ran"),
        rec(2_000, "sys:run", "irreversible", "allow", "ran"),
        denied,
    ]
}

#[test]
fn a_term_matches_any_column_a_person_would_search_by() {
    // The tool…
    let by_tool = listing(&sample(), 0, "sys:run");
    assert!(by_tool.contains("sys:run"));
    assert!(!by_tool.contains("gmail:send"), "{by_tool}");
    // …the tier…
    assert!(listing(&sample(), 0, "read").contains("sys:list_dir"));
    // …the outcome…
    assert!(listing(&sample(), 0, "deny").contains("gmail:send"));
    // …and the note, which is where a failure says what went wrong.
    assert!(listing(&sample(), 0, "no human").contains("gmail:send"));
}

#[test]
fn the_term_is_case_insensitive() {
    assert!(listing(&sample(), 0, "GMAIL").contains("gmail:send"));
}

#[test]
fn the_title_names_the_filter_so_a_short_list_is_not_mistaken_for_the_whole_one() {
    let out = listing(&sample(), 0, "sys:run");
    assert!(out.contains("matching"), "{out}");
    assert!(out.contains("sys:run"), "{out}");
}

/// "There is no history" and "your search found none of it" are different
/// answers, and only one of them means you typed it wrong.
#[test]
fn a_filter_that_matches_nothing_says_so_differently_from_an_empty_ledger() {
    let no_match = listing(&sample(), 0, "kubernetes");
    assert!(no_match.contains("No call matches"), "{no_match}");
    assert!(no_match.contains("3 call(s) recorded"), "{no_match}");
    assert!(
        no_match.contains("/tools with no term"),
        "the way back: {no_match}"
    );

    let empty = listing(&[], 0, "kubernetes");
    assert!(empty.contains("Nothing yet"), "{empty}");
}

/// The tail cap counts MATCHES, not the whole ledger — a filter over a huge
/// ledger that hid rows it never matched would be lying about what it hid.
#[test]
fn the_cap_counts_matches_not_the_whole_ledger() {
    let mut many: Vec<Record> = (0..MAX_ROWS as u64 + 10)
        .map(|i| rec(i, "sys:run", "read", "allow", "ran"))
        .collect();
    many.extend((0..50).map(|i| rec(i, "fs:read", "read", "allow", "ran")));
    let out = listing(&many, 0, "fs:read");
    assert!(!out.contains("not shown"), "50 matches fit: {out}");
    // Skip the heading, which names the filter and so contains it too.
    let rows = out.lines().filter(|l| l.starts_with("00:")).count();
    assert_eq!(rows, 50);
}

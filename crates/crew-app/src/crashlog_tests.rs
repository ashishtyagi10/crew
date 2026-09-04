use super::*;

/// The record carries every field needed to act on a crash — without them
/// the log is no better than the silence it replaces.
#[test]
fn record_names_place_message_and_thread() {
    let r = record(
        "2026-07-27 18:05:31",
        "0.7.0",
        "main",
        "index out of bounds",
        "src/paneview.rs:42:9",
        "  0: crew::main",
    );
    assert!(r.contains("2026-07-27 18:05:31"));
    assert!(r.contains("0.7.0"));
    assert!(r.contains("main"));
    assert!(r.contains("index out of bounds"));
    assert!(r.contains("src/paneview.rs:42:9"));
    assert!(r.contains("crew::main"));
}

/// The summary is flashed in the one-line status bar, so a multi-line
/// panic (every `assert_eq!` failure) must not smear across the chrome.
#[test]
fn summary_is_one_line_and_bounded() {
    let s = summary("now", "assertion failed: a == b\n  left: 1\n right: 2");
    assert_eq!(s, "now — assertion failed: a == b");
    assert!(!s.contains('\n'));

    let long = "x".repeat(500);
    let s = summary("now", &long);
    assert!(
        s.chars().count() < 140,
        "unbounded summary: {} chars",
        s.chars().count()
    );
    // …and the cut is marked: the one line saying why crew died used to
    // end mid-sentence looking complete.
    assert!(s.ends_with('\u{2026}'), "{s}");
}

#[test]
fn summary_survives_an_empty_message() {
    assert_eq!(summary("now", ""), "now — ");
}

/// A crash loop must not fill the disk: an oversized log is dropped rather
/// than appended to forever.
#[test]
fn append_truncates_past_the_cap() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("crash.log");
    std::fs::write(&p, vec![b'x'; (MAX_LOG_BYTES + 1) as usize]).unwrap();
    append_capped(&p, "fresh record");
    let got = std::fs::read_to_string(&p).unwrap();
    assert_eq!(got, "fresh record", "oversized log should be replaced");
}

#[test]
fn append_keeps_earlier_records_under_the_cap() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("crash.log");
    append_capped(&p, "first\n");
    append_capped(&p, "second\n");
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "first\nsecond\n");
}

/// The note has to point somewhere; "it crashed" with no location is the
/// same dead end as the original silence.
#[test]
fn crash_note_mentions_the_summary() {
    assert!(crash_note("boom").contains("boom"));
}

/// A clean previous run must not produce a crash note.
#[test]
fn no_marker_means_no_report() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(take_report_at(Some(dir.path().join("absent"))), None);
    assert_eq!(take_report_at(None), None);
}

/// The crash is announced once. A marker left behind would re-report the
/// same long-dead panic on every launch forever after.
#[test]
fn report_is_consumed_exactly_once() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("last-crash");
    std::fs::write(&p, "2026-07-27 18:05:31 — boom\n").unwrap();

    assert_eq!(
        take_report_at(Some(p.clone())),
        Some("2026-07-27 18:05:31 — boom".to_string()),
        "first launch after a crash should report it"
    );
    assert_eq!(
        take_report_at(Some(p.clone())),
        None,
        "second launch should be silent"
    );
    assert!(!p.exists(), "marker should be gone");
}

/// An empty/whitespace marker is not a crash report — and must still be
/// cleared, or it jams the check permanently.
#[test]
fn blank_marker_reports_nothing_and_is_cleared() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("last-crash");
    std::fs::write(&p, "  \n").unwrap();
    assert_eq!(take_report_at(Some(p.clone())), None);
    assert!(!p.exists(), "blank marker should still be removed");
}

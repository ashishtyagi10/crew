use super::*;

#[test]
fn a_program_that_brackets_its_paste_is_never_asked_about() {
    assert!(!needs_confirm("one\ntwo\nthree", true));
    assert!(!needs_confirm("anything at all", true));
}

/// The case worth asking about: a newline with something after it, into a
/// program that will run each line as it arrives.
#[test]
fn an_unbracketed_multi_line_paste_is_held() {
    assert!(needs_confirm("rm -rf /tmp/x\necho done", false));
    assert!(needs_confirm("a\nb\nc\n", false));
}

/// One trailing newline is the common harmless case — copying a line out of a
/// file takes its terminator — and holding it would train people to confirm
/// everything.
#[test]
fn a_single_line_with_its_terminator_is_not_held() {
    assert!(!needs_confirm("cargo build\n", false));
    assert!(!needs_confirm("cargo build\r\n", false));
    assert!(!needs_confirm("cargo build", false));
    assert!(!needs_confirm("", false));
}

#[test]
fn the_count_is_what_would_run() {
    assert_eq!(line_count("a\nb\nc"), 3);
    assert_eq!(line_count("a\nb\nc\n"), 3);
    assert_eq!(line_count("one"), 1);
    assert_eq!(line_count(""), 0);
}

#[test]
fn a_held_paste_is_returned_once() {
    let now = Instant::now();
    let mut held = Held::default();
    assert_eq!(held.take(now), None, "nothing held is nothing to send");
    held.hold("a\nb", now);
    assert_eq!(held.take(now).as_deref(), Some("a\nb"));
    assert_eq!(held.take(now), None, "the same paste went twice");
}

/// A confirmation you have forgotten giving is not a confirmation.
#[test]
fn a_stale_hold_is_dropped_rather_than_sent() {
    let now = Instant::now();
    let mut held = Held::default();
    held.hold("a\nb", now);
    assert_eq!(held.take(now + HOLD + Duration::from_secs(1)), None);
    // …and it is gone, not merely refused.
    held.hold("a\nb", now);
    assert_eq!(held.take(now + HOLD), Some("a\nb".to_string()));
}

#[test]
fn the_newer_clipboard_replaces_the_older_hold() {
    let now = Instant::now();
    let mut held = Held::default();
    held.hold("first\nblock", now);
    held.hold("second\nblock", now);
    assert_eq!(held.take(now).as_deref(), Some("second\nblock"));
    held.hold("x\ny", now);
    held.clear();
    assert_eq!(held.take(now), None);
}

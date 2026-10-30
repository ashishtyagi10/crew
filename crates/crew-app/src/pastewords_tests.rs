//! A paste carries words, never keys.
use crate::session::wrap_paste;

/// The bracket's own end marker inside a paste can no longer end it early.
#[test]
fn a_pasted_end_marker_cannot_close_the_bracket() {
    let hostile = "echo safe\x1b[201~\rrm -rf ~/x\r";
    let sent = String::from_utf8(wrap_paste(hostile, true)).unwrap();
    assert_eq!(sent.matches("\x1b[201~").count(), 1, "{sent:?}");
    assert!(sent.ends_with("\x1b[201~"), "the real one, last: {sent:?}");
    assert!(!sent[6..sent.len() - 6].contains('\x1b'), "{sent:?}");
}

#[test]
fn tabs_and_line_breaks_survive_and_other_controls_do_not() {
    let text = "a\tb\nc\x07d\x7fe\u{9b}31mf";
    let sent = String::from_utf8(wrap_paste(text, false)).unwrap();
    assert_eq!(sent, "a\tb\rcde31mf");
}

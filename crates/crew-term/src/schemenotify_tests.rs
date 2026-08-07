use super::*;

#[test]
fn decset_2031_enables_and_decrst_disables() {
    let mut s = SchemeNotify::default();
    assert!(!s.enabled());
    assert_eq!(s.feed(b"\x1b[?2031h", true), "");
    assert!(s.enabled());
    assert_eq!(s.feed(b"\x1b[?2031l", true), "");
    assert!(!s.enabled());
    // Multi-param DECSET (alt screen + 2031 in one sequence) still counts.
    s.feed(b"\x1b[?1049;2031h", true);
    assert!(s.enabled());
}

#[test]
fn sequences_split_across_chunks_reassemble() {
    let mut s = SchemeNotify::default();
    // The PTY hands us the sequence one byte at a time.
    for &b in b"\x1b[?2031h" {
        s.feed(&[b], true);
    }
    assert!(s.enabled(), "split DECSET must still enable");
    // A query split mid-params still answers.
    let mut reply = String::new();
    reply.push_str(&s.feed(b"\x1b[?9", true));
    reply.push_str(&s.feed(b"96n", true));
    assert_eq!(reply, "\x1b[?997;1n");
}

#[test]
fn decrqm_reports_set_or_reset() {
    let mut s = SchemeNotify::default();
    assert_eq!(s.feed(b"\x1b[?2031$p", true), "\x1b[?2031;2$y", "reset");
    s.feed(b"\x1b[?2031h", true);
    assert_eq!(s.feed(b"\x1b[?2031$p", true), "\x1b[?2031;1$y", "set");
}

#[test]
fn query_996_reports_the_current_scheme() {
    let mut s = SchemeNotify::default();
    assert_eq!(s.feed(b"\x1b[?996n", true), "\x1b[?997;1n", "dark = 1");
    assert_eq!(s.feed(b"\x1b[?996n", false), "\x1b[?997;2n", "light = 2");
    // Works without ever enabling 2031 — it's a one-shot question.
    assert!(!s.enabled());
}

#[test]
fn unrelated_sequences_and_text_are_ignored() {
    let mut s = SchemeNotify::default();
    let noise: &[u8] =
        b"plain text \x1b[31mred\x1b[0m \x1b[?1049h \x1b[?25l \x1b]0;title\x07 \x1b[6n";
    assert_eq!(s.feed(noise, true), "");
    assert!(!s.enabled(), "2031 must not trip on other private modes");
    // A pathological parameter flood doesn't hoard bytes or match.
    let flood = format!("\x1b[?{}h", "2031".repeat(40));
    assert_eq!(s.feed(flood.as_bytes(), true), "");
    assert!(!s.enabled());
}

#[test]
fn scheme_report_encodes_dark_and_light() {
    assert_eq!(scheme_report(true), "\x1b[?997;1n");
    assert_eq!(scheme_report(false), "\x1b[?997;2n");
}

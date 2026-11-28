use super::*;

fn fed(chunks: &[&[u8]]) -> PointerShape {
    let mut s = PointerShape::default();
    for c in chunks {
        s.feed(c);
    }
    s
}

#[test]
fn bel_and_st_both_end_the_sequence() {
    assert_eq!(fed(&[b"\x1b]22;pointer\x07"]).name(), Some("pointer"));
    assert_eq!(fed(&[b"\x1b]22;text\x1b\\"]).name(), Some("text"));
}

#[test]
fn a_sequence_split_across_reads_reassembles() {
    let mut s = PointerShape::default();
    for b in b"\x1b]22;grab\x1b\\" {
        s.feed(&[*b]);
    }
    assert_eq!(s.name(), Some("grab"));
}

#[test]
fn empty_default_and_ris_clear() {
    for clear in [&b"\x1b]22;\x07"[..], b"\x1b]22;default\x07", b"\x1bc"] {
        let mut s = fed(&[b"\x1b]22;pointer\x07"]);
        s.feed(clear);
        assert_eq!(s.name(), None, "{clear:?}");
    }
}

#[test]
fn surrounding_output_and_other_oscs_are_ignored() {
    let s = fed(&[b"hi\x1b]0;title\x07\x1b[31m\x1b]22;crosshair\x07rest"]);
    assert_eq!(s.name(), Some("crosshair"));
    let s = fed(&[b"\x1b]222;pointer\x07\x1b]2;22;x\x07"]);
    assert_eq!(s.name(), None);
}

#[test]
fn queries_and_overlong_or_cancelled_sequences_leave_it_alone() {
    let mut s = fed(&[b"\x1b]22;move\x07"]);
    s.feed(b"\x1b]22;?__current__\x07\x1b]22;>wait\x07");
    assert_eq!(s.name(), Some("move"));
    s.feed(format!("\x1b]22;{}\x07", "a".repeat(500)).as_bytes());
    assert_eq!(s.name(), Some("move"));
    s.feed(b"\x1b]22;wait\x18");
    s.feed(b"\x07");
    assert_eq!(s.name(), Some("move"));
}

#[test]
fn kitty_equals_form_sets() {
    assert_eq!(fed(&[b"\x1b]22;=help\x07"]).name(), Some("help"));
}

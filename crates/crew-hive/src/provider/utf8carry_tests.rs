use super::*;

/// `s` fed in two reads split at every byte, joined back.
fn split_everywhere(s: &str) -> Vec<String> {
    (0..=s.len())
        .map(|at| {
            let mut c = Utf8Carry::default();
            let (a, b) = s.as_bytes().split_at(at);
            let mut out = c.push(a);
            out.push_str(&c.push(b));
            out.push_str(&c.finish());
            out
        })
        .collect()
}

#[test]
fn a_character_split_between_two_reads_arrives_whole() {
    for s in ["a—b", "漢字", "ok 🙂 ok", "é"] {
        for got in split_everywhere(s) {
            assert_eq!(got, s);
        }
    }
}

#[test]
fn the_unfinished_tail_waits_and_the_rest_comes_out_now() {
    let mut c = Utf8Carry::default();
    let dash = "—".as_bytes();
    assert_eq!(c.push(&[b'a', dash[0]]), "a");
    assert_eq!(c.push(&dash[1..2]), "");
    assert_eq!(c.push(&[dash[2], b'b']), "—b");
    assert_eq!(c.finish(), "");
}

#[test]
fn bytes_that_are_not_text_are_marked_not_held() {
    let mut c = Utf8Carry::default();
    // A lone continuation byte and an invalid lead never start a character.
    assert_eq!(c.push(&[b'x', 0x80]), "x\u{FFFD}");
    assert_eq!(c.push(&[0xFF]), "\u{FFFD}");
    // A character cut off by the end of the stream is still shown.
    assert_eq!(c.push(&[0xE2, 0x80]), "");
    assert_eq!(c.finish(), "\u{FFFD}");
}

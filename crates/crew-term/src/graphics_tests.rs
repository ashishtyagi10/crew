//! The splitter's contract: every byte that is not part of a graphics
//! sequence reaches the parser, in order, exactly once — and a sequence is
//! recognised however the reader thread happened to cut the stream.
use super::*;

fn b64(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Feed `chunks` in order; return the text the parser would have seen and the
/// images that came out.
fn run(chunks: &[&[u8]]) -> (Vec<u8>, Vec<ImageCmd>) {
    let mut sc = GraphicsScanner::default();
    let (mut text, mut imgs) = (Vec::new(), Vec::new());
    for c in chunks {
        for seg in sc.feed(c) {
            match seg {
                Seg::Bytes(b) => text.extend_from_slice(b),
                Seg::Esc => text.push(0x1b),
                Seg::Image(i) => imgs.push(i),
            }
        }
    }
    (text, imgs)
}

fn seq(payload: &[u8]) -> Vec<u8> {
    format!("\x1b_Ga=T,f=100;{}\x1b\\", b64(payload)).into_bytes()
}

#[test]
fn a_stream_with_no_pictures_in_it_passes_through_byte_for_byte() {
    let s = b"hello \x1b[31mworld\x1b[0m \x1b]0;title\x07 done";
    let (text, imgs) = run(&[s]);
    assert_eq!(text, s, "the parser must see the stream unchanged");
    assert!(imgs.is_empty());
}

#[test]
fn the_text_around_a_picture_reaches_the_parser_in_order() {
    let mut stream = b"before".to_vec();
    stream.extend(seq(b"PNGDATA"));
    stream.extend(b"after");
    let (text, imgs) = run(&[&stream]);
    assert_eq!(text, b"beforeafter", "the sequence itself is not text");
    assert_eq!(imgs.len(), 1);
    assert_eq!(imgs[0].data, b"PNGDATA");
}

/// This is the whole reason the stream is split rather than sniffed: the
/// caller feeds the parser one segment at a time, so the cursor is where the
/// program left it before the picture is placed.
#[test]
fn the_bytes_before_a_picture_are_a_segment_of_their_own() {
    let mut stream = b"a\r\n".to_vec();
    stream.extend(seq(b"x"));
    let mut sc = GraphicsScanner::default();
    let segs = sc.feed(&stream);
    assert!(matches!(segs.first(), Some(Seg::Bytes(b"a\r\n"))));
    assert!(matches!(segs.get(1), Some(Seg::Image(_))));
}

/// The reader thread cuts on buffer boundaries, not on escape sequences.
#[test]
fn a_sequence_split_across_reads_is_still_one_picture() {
    let mut stream = b"x".to_vec();
    stream.extend(seq(b"SPLITME"));
    stream.extend(b"y");
    for at in 1..stream.len() {
        let (text, imgs) = run(&[&stream[..at], &stream[at..]]);
        assert_eq!(text, b"xy", "split at {at} lost or duplicated text");
        assert_eq!(imgs.len(), 1, "split at {at} lost the picture");
        assert_eq!(imgs[0].data, b"SPLITME", "split at {at}");
    }
}

/// `m=1` says the payload continues in the next sequence. Producers chunk at
/// 4096 base64 bytes, so nearly every real screenshot arrives this way.
#[test]
fn a_chunked_picture_is_joined_into_one() {
    let a = format!("\x1b_Ga=T,f=100,m=1;{}\x1b\\", b64(b"AAAA"));
    let b = format!("\x1b_Gm=1;{}\x1b\\", b64(b"BBBB"));
    let c = format!("\x1b_Gm=0;{}\x1b\\", b64(b"CCCC"));
    let (text, imgs) = run(&[a.as_bytes(), b.as_bytes(), c.as_bytes()]);
    assert!(text.is_empty());
    assert_eq!(imgs.len(), 1, "one picture, not three");
    assert_eq!(imgs[0].data, b"AAAABBBBCCCC");
    assert_eq!(
        imgs[0].format, 100,
        "the first chunk's keys are the image's"
    );
}

/// A BEL-terminated sequence is the same sequence; several producers write it.
#[test]
fn bel_terminates_a_sequence_as_well_as_st() {
    let s = format!("\x1b_Ga=T,f=100;{}\x07tail", b64(b"z"));
    let (text, imgs) = run(&[s.as_bytes()]);
    assert_eq!(text, b"tail");
    assert_eq!(imgs.len(), 1);
}

/// An ESC that is not the start of an APC belongs to the parser — including
/// two in a row, which is where a naive hold-one-byte scanner drops one.
#[test]
fn an_escape_that_is_not_a_picture_is_handed_over_whole() {
    let (text, imgs) = run(&[b"\x1b\x1b[A\x1bP+q\x1b\\"]);
    assert_eq!(text, b"\x1b\x1b[A\x1bP+q\x1b\\");
    assert!(imgs.is_empty());
}

/// An APC that never terminates must not swallow the session's output — but
/// it also must not be spat out as text, or a truncated picture becomes a
/// screenful of base64.
#[test]
fn an_unterminated_sequence_holds_rather_than_leaking() {
    let (text, imgs) = run(&[b"seen\x1b_Ga=T;AAAA", b"BBBB"]);
    assert_eq!(text, b"seen");
    assert!(imgs.is_empty());
}

/// An APC that is not a graphics command (kitty's own keyboard-protocol
/// probes, tmux passthrough) is dropped, not printed.
#[test]
fn a_non_graphics_apc_is_swallowed_quietly() {
    let (text, imgs) = run(&[b"a\x1b_qsomething\x1b\\b"]);
    assert_eq!(text, b"ab");
    assert!(imgs.is_empty());
}

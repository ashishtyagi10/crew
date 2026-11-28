//! Sixel in the splitter: found however the stream is cut, aborted by an
//! escape that is not its terminator, and never mistaken for any other DCS.
use super::*;

/// What the parser would have seen, and the pictures that came out.
fn run(chunks: &[&[u8]]) -> (Vec<u8>, Vec<ImageCmd>) {
    let mut sc = GraphicsScanner::default();
    let (mut text, mut imgs) = (Vec::new(), Vec::new());
    for c in chunks {
        for seg in sc.feed(c) {
            match seg {
                Seg::Bytes(b) => text.extend_from_slice(b),
                Seg::Held(b) => text.extend_from_slice(&b),
                Seg::Image(i) => imgs.push(i),
            }
        }
    }
    (text, imgs)
}

/// The same stream cut at every offset, and one byte at a time: all must
/// agree with the uncut run.
fn every_cut(s: &[u8]) -> (Vec<u8>, Vec<ImageCmd>) {
    let whole = run(&[s]);
    for at in 0..=s.len() {
        assert_eq!(run(&[&s[..at], &s[at..]]), whole, "cut at {at}");
    }
    let bytes: Vec<&[u8]> = s.chunks(1).collect();
    assert_eq!(run(&bytes), whole, "byte by byte");
    whole
}

const SIXEL: &[u8] = b"pre\x1bP0;1;0q\"1;1;4;6#1;2;100;0;0#1!4~-~\x1b\\post";

#[test]
fn a_sixel_is_cut_out_and_its_body_kept_whole() {
    let (text, imgs) = every_cut(SIXEL);
    assert_eq!(text, b"prepost");
    assert_eq!(imgs.len(), 1);
    assert!(imgs[0].sixel && imgs[0].displays() && imgs[0].id == 0);
    assert!(imgs[0].supported(), "or place_image would refuse it");
    assert_eq!(imgs[0].data, b"0;1;0q\"1;1;4;6#1;2;100;0;0#1!4~-~");
}

#[test]
fn the_bytes_before_a_sixel_are_a_segment_of_their_own() {
    let mut sc = GraphicsScanner::default();
    let segs = sc.feed(SIXEL);
    assert!(matches!(segs.first(), Some(Seg::Bytes(b"pre"))));
    assert!(matches!(segs.get(1), Some(Seg::Image(_))));
    assert!(matches!(segs.get(2), Some(Seg::Bytes(b"post"))));
}

/// XTGETTCAP asks `ESC P + q <hex> ESC \`: the `q` is there, the digits are
/// not, and the parser (and `modelcaps`) must get every byte.
#[test]
fn xtgettcap_is_not_a_sixel_however_it_is_cut() {
    let s = b"a\x1bP+q544e\x1b\\b";
    let (text, imgs) = every_cut(s);
    assert_eq!(text, s);
    assert!(imgs.is_empty());
}

#[test]
fn other_dcs_strings_reach_the_parser_untouched() {
    for s in [
        &b"a\x1bP$qm\x1b\\b"[..],
        b"a\x1bPtmux;\x1b\x1b[0m\x1b\\b",
        b"a\x1bP>|crew 1\x1b\\b",
        b"a\x1bP1000p\x1b\\b",
        b"a\x1bP\x1b\\b",
        b"a\x1bP1;2;3;4;5;6;7;8;9;1;2;3;4;5;6;7;8;9q~\x1b\\b",
        b"\x1b\x1bP+q\x1b\\",
    ] {
        let (text, imgs) = every_cut(s);
        assert_eq!(text, s, "{s:?}");
        assert!(imgs.is_empty(), "{s:?}");
    }
}

/// `cat img.six` then Ctrl-C: the next thing on the pty is the shell's own
/// escape, which must reach the parser, not die inside the picture.
#[test]
fn an_escape_that_is_not_the_terminator_aborts_the_sixel() {
    let s = b"\x1bP0;0;0q#1~~~\x1b[31mred";
    let (text, imgs) = every_cut(s);
    assert!(imgs.is_empty(), "the half picture is dropped");
    assert_eq!(text, b"\x1b[31mred");
    // Two escapes in a row: the second one still starts a sequence.
    let (text, _) = every_cut(b"\x1bPq~\x1b\x1b[0m");
    assert_eq!(text, b"\x1b\x1b[0m");
    let (text, imgs) = every_cut(b"\x1bPq~\x1b\x1bPq~\x1b\\z");
    assert_eq!(text, b"\x1bz", "the first aborts, the second completes");
    assert_eq!(imgs.len(), 1);
}

#[test]
fn bel_does_not_end_a_sixel() {
    let (text, imgs) = every_cut(b"\x1bPq~\x07~\x1b\\x");
    assert_eq!(text, b"x");
    assert_eq!(imgs[0].data, b"q~~");
}

#[test]
fn a_sixel_that_never_ends_is_dropped_not_buffered_forever() {
    let mut s = b"\x1bPq".to_vec();
    s.resize(s.len() + 17 * 1024 * 1024, b'~');
    s.extend(b"\x1b\\ok\x1bPq~\x1b\\");
    let (text, imgs) = run(&[&s]);
    assert_eq!(text, b"ok");
    assert_eq!(imgs.len(), 1, "the next picture still works");
    assert_eq!(imgs[0].data, b"q~");
}

/// `sixel` is a flag no kitty key reaches, so a kitty probe is answered as it
/// always was.
#[test]
fn kitty_cannot_name_a_sixel() {
    for f in ["0", "1", "6", "100", "24", "32", "999"] {
        let body = format!("Ga=q,f={f};");
        let cmd = ImageCmd::parse(body.as_bytes()).unwrap();
        assert!(!cmd.sixel, "f={f}");
        assert_eq!(cmd.supported(), matches!(f, "100" | "24" | "32"), "f={f}");
    }
}

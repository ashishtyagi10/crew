//! The command grammar: what a producer actually writes, and what must never
//! be guessed at.
use super::*;

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn parse(body: &str) -> Option<ImageCmd> {
    ImageCmd::parse(body.as_bytes())
}

#[test]
fn the_form_kitty_icat_writes() {
    let c = parse(&format!("Gf=100,a=T,c=20,r=10;{}", b64(b"\x89PNG..."))).expect("parses");
    assert_eq!(c.format, 100);
    assert_eq!(c.action, b'T');
    assert_eq!(c.cells, (20, 10));
    assert_eq!(c.data, b"\x89PNG...");
    assert!(c.displays() && !c.deletes());
    assert!(!c.more);
}

/// Every key is optional, and the protocol's own defaults are what a bare
/// command means — not zero.
#[test]
fn an_empty_key_list_takes_the_protocols_defaults() {
    let c = parse(&format!("G;{}", b64(b"x"))).expect("parses");
    assert_eq!((c.action, c.format, c.medium), (b't', 32, b'd'));
    assert!(!c.displays(), "transmit-only must not draw anything");
}

/// A raw-pixel transmission carries no header, so its size is in the keys and
/// nowhere else.
#[test]
fn raw_pixels_carry_their_size_in_the_keys() {
    let c = parse(&format!("Gf=24,s=4,v=2,a=T;{}", b64(&[7u8; 24]))).expect("parses");
    assert_eq!((c.format, c.px), (24, (4, 2)));
    assert_eq!(c.data.len(), 24, "4×2 RGB");
}

#[test]
fn a_file_transmission_reads_its_payload_as_a_path() {
    let c = parse(&format!("Ga=T,t=f,f=100;{}", b64(b"/tmp/shot.png"))).expect("parses");
    assert_eq!(c.path(), Some(std::path::PathBuf::from("/tmp/shot.png")));
    // …and a direct transmission's payload is never treated as one, or a
    // picture's first bytes become a filesystem lookup.
    let direct = parse(&format!("Ga=T,f=100;{}", b64(b"/tmp/shot.png"))).expect("parses");
    assert_eq!(direct.path(), None);
}

#[test]
fn deletion_is_recognised_as_itself() {
    let c = parse("Ga=d;").expect("parses");
    assert!(c.deletes() && !c.displays());
}

/// The protocol keeps growing; a key crew has never heard of must not cost
/// the picture it was attached to.
#[test]
fn an_unknown_key_does_not_lose_the_picture() {
    let c = parse(&format!("Ga=T,f=100,z=-1,q=2,X=9;{}", b64(b"png"))).expect("parses");
    assert!(c.displays());
    assert_eq!(c.data, b"png");
}

/// Anything that is not a `G` command, or not decodable, produces nothing —
/// and the sequence is already out of the byte stream, so the failure is a
/// missing picture, never a screenful of escape text.
#[test]
fn a_command_that_cannot_be_read_yields_nothing() {
    assert_eq!(parse("Xa=T;aGVsbG8="), None, "not a graphics command");
    assert_eq!(parse("Ga=T;not valid base64!!"), None);
    assert_eq!(parse("Ga=T,f=abc;"), None, "a non-numeric count");
    assert_eq!(parse("Ga=T,broken;"), None, "a key with no value");
}

#[test]
fn a_chunked_transmission_says_so() {
    let first = parse(&format!("Ga=T,f=100,m=1;{}", b64(b"aa"))).expect("parses");
    let last = parse(&format!("Gm=0;{}", b64(b"bb"))).expect("parses");
    assert!(first.more);
    assert!(!last.more);
}

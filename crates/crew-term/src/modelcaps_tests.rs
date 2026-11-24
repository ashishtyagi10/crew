//! XTGETTCAP as a program on the pty asks it, and as neovim reads the answer.
use super::*;
use crate::model::{GridSize, HeadlessTerm, TermModel};

fn feed(c: &mut Caps, bytes: &[u8]) -> String {
    bytes.iter().filter_map(|&b| c.step(b)).collect()
}

fn ask(bytes: &[u8]) -> String {
    feed(&mut Caps::default(), bytes)
}

/// `RGB` is found, with xterm's `8/8/8` ("382F382F38").
#[test]
fn rgb_answers_found_with_its_value() {
    assert_eq!(
        ask(b"\x1bP+q524742\x1b\\"),
        "\x1bP1+r524742=382F382F38\x1b\\"
    );
}

#[test]
fn unknown_name_answers_not_found_with_the_name() {
    // "xyz"
    assert_eq!(ask(b"\x1bP+q78797A\x1b\\"), "\x1bP0+r78797A\x1b\\");
}

/// Booleans come back found with no `=`: neovim reads "found" from the
/// status digit, which is all it needs to turn on 24-bit colour.
#[test]
fn tc_is_a_boolean() {
    assert_eq!(ask(b"\x1bP+q5463\x1b\\"), "\x1bP1+r5463\x1b\\");
}

/// neovim 0.10 sends `Tc;RGB;setrgbf;setrgbb` in one request and reads one
/// pair per reply.
#[test]
fn several_names_answer_one_reply_each_in_order() {
    let out = ask(b"\x1bP+q5463;78;524742\x1b\\");
    assert_eq!(
        out,
        "\x1bP1+r5463\x1b\\\x1bP0+r78\x1b\\\x1bP1+r524742=382F382F38\x1b\\"
    );
}

#[test]
fn a_request_split_across_reads_still_answers() {
    let mut c = Caps::default();
    assert_eq!(feed(&mut c, b"ab\x1bP+q5247"), "");
    assert_eq!(feed(&mut c, b"42\x1b"), "");
    assert_eq!(feed(&mut c, b"\\"), "\x1bP1+r524742=382F382F38\x1b\\");
}

#[test]
fn lowercase_and_uppercase_hex_both_read() {
    // "Ms" — the OSC 52 write, as terminfo source text.
    let want = format!("\x1bP1+r4D73={}\x1b\\", hex("\\E]52;%p1%s;%p2%s\\007"));
    assert_eq!(ask(b"\x1bP+q4d73\x1b\\"), want);
    assert_eq!(ask(b"\x1bP+q4D73\x1b\\"), want);
}

/// Odd lengths and non-digits are "not found", never a panic; bytes that are
/// not hex digits are not echoed, or a `cat`ed file could type into the shell.
#[test]
fn malformed_hex_answers_not_found() {
    assert_eq!(ask(b"\x1bP+q524\x1b\\"), "\x1bP0+r524\x1b\\");
    assert_eq!(ask(b"\x1bP+qzz\x1b\\"), "\x1bP0+r\x1b\\");
    assert_eq!(ask(b"\x1bP+q\r\nrm\x1b\\"), "\x1bP0+r\x1b\\");
    assert_eq!(ask(b"\x1bP+q\xff\xfe\x1b\\"), "\x1bP0+r\x1b\\");
    // Empty names (`;;`, a bare request) are skipped, not answered.
    assert_eq!(ask(b"\x1bP+q;5463;\x1b\\"), "\x1bP1+r5463\x1b\\");
    assert_eq!(ask(b"\x1bP+q\x1b\\"), "");
}

/// Other DCS strings, an aborted request and an overlong one stay silent,
/// and the scanner recovers for the next real request.
#[test]
fn near_misses_stay_silent_and_recover() {
    let mut c = Caps::default();
    // DECRQSS (`$ q`), sixel (`q` with no `+`), a cancelled and an
    // ESC-interrupted request, then one too long to be a query.
    assert_eq!(feed(&mut c, b"\x1bP$qm\x1b\\\x1bPq#0\x1b\\"), "");
    assert_eq!(feed(&mut c, b"\x1bP+q5463\x18\x1b\\"), "");
    assert_eq!(feed(&mut c, b"\x1bP+q5463\x1b[0m\x1b\\"), "");
    let long = [b"\x1bP+q".as_slice(), &[b'4'; MAX_BODY + 10], b"\x1b\\"].concat();
    assert_eq!(feed(&mut c, &long), "");
    assert_eq!(feed(&mut c, b"\x1bP+q5463\x1b\\"), "\x1bP1+r5463\x1b\\");
}

/// Every advertised name answers found, and its value decodes back to the
/// terminfo text — a typo in the table would fail here, not in neovim.
#[test]
fn every_cap_in_the_table_answers() {
    for (k, v) in CAPS {
        let got = ask(format!("\x1bP+q{}\x1b\\", hex(k)).as_bytes());
        let want = match v {
            Some(v) => format!("\x1bP1+r{}={}\x1b\\", hex(k), hex(v)),
            None => format!("\x1bP1+r{}\x1b\\", hex(k)),
        };
        assert_eq!(got, want, "{k}");
    }
    assert_eq!(unhex(hex("TN").as_bytes()).as_deref(), Some(&b"TN"[..]));
}

/// Through the whole terminal: the reply goes out once (vte and the graphics
/// scanner, which also see `ESC P`, add nothing) and the grid stays blank.
#[test]
fn through_the_term_the_reply_is_written_once() {
    let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
    t.feed(b"\x1bP+q544E\x1b\\");
    let want = format!("\x1bP1+r544E={}\x1b\\", hex("xterm-256color"));
    assert_eq!(t.take_replies(), Some(want));
    assert!(t.cells(false).iter().all(|c| c.c == ' '));
}

/// Asked alongside XTVERSION, both answer in the order asked.
#[test]
fn with_xtversion_the_asked_order_holds() {
    let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
    t.feed(b"\x1bP+q5463\x1b\\\x1b[>q");
    let want = format!("\x1bP1+r5463\x1b\\{}", super::super::xtversion_reply());
    assert_eq!(t.take_replies(), Some(want));
}

/// What `setrgbb` advertises is what crew draws: SGR `48;2` paints the
/// exact background (`38;2` is proven in `model_tests`).
#[test]
fn setrgbb_form_paints_the_exact_background() {
    let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
    t.feed(b"\x1b[48;2;12;200;99mx");
    let cells = t.cells(false);
    let x = cells.iter().find(|c| c.c == 'x').expect("drawn");
    assert_eq!(x.bg, (12, 200, 99));
}

//! DECRQSS through the parser: the reply reports the state at the request.
use crate::{GridSize, HeadlessTerm, TermModel};

fn asked(bytes: &[u8]) -> Option<String> {
    let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
    t.feed(bytes);
    t.take_replies()
}

fn ok(v: &str) -> Option<String> {
    Some(format!("\x1bP1$r{v}\x1b\\"))
}

/// nvim 0.10's truecolor probe, set and ask in one write: the reply must end
/// with the r;g;b it set.
#[test]
fn nvim_reads_back_the_direct_background_it_just_set() {
    assert_eq!(
        asked(b"\x1b[48;2;1;2;3m\x1bP$qm\x1b\\"),
        ok("0;48;2;1;2;3m")
    );
}

#[test]
fn sgr_reports_attributes_and_colours_from_a_reset() {
    assert_eq!(asked(b"\x1bP$qm\x1b\\"), ok("0m"));
    let got = asked(b"\x1b[1;3;9;31;48;5;208m\x1bP$qm\x1b\\");
    assert_eq!(got, ok("0;1;3;9;31;48;5;208m"));
    let bright = asked(b"\x1b[7;94;38:2::9:8:7m\x1bP$qm\x1b\\");
    assert_eq!(bright, ok("0;7;38;2;9;8;7m"));
    assert_eq!(
        asked(b"\x1b[4:3;58;5;1m\x1bP$qm\x1b\\"),
        ok("0;4:3;58;5;1m")
    );
}

/// The state AT the request: what the program wrote after the ask, in the
/// same read, is not reported.
#[test]
fn the_reply_is_the_state_at_the_request_not_at_the_end_of_the_read() {
    assert_eq!(asked(b"\x1b[31m\x1bP$qm\x1b\\\x1b[32m"), ok("0;31m"));
}

/// vim's `t_RS`: the cursor's DECSCUSR number, odd when it blinks.
#[test]
fn the_cursor_style_reports_its_decscusr_number() {
    for (set, want) in [
        ("1", "1"),
        ("2", "2"),
        ("3", "3"),
        ("4", "4"),
        ("5", "5"),
        ("6", "6"),
    ] {
        let got = asked(format!("\x1b[{set} q\x1bP$q q\x1b\\").as_bytes());
        assert_eq!(got, ok(&format!("{want} q")), "DECSCUSR {set}");
    }
}

#[test]
fn a_request_split_across_reads_still_answers() {
    let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
    t.feed(b"\x1b[5 q\x1bP$");
    t.feed(b"q q\x1b");
    t.feed(b"\\");
    assert_eq!(t.take_replies(), ok("5 q"));
}

/// A setting crew cannot read — the scroll margins — or does not know is
/// answered "invalid", never guessed and never echoed back.
#[test]
fn an_unknown_setting_is_answered_invalid_without_echo() {
    for asked_for in ["r", "\"p", "rm -rf ~\r"] {
        let got = asked(format!("\x1bP$q{asked_for}\x1b\\").as_bytes());
        assert_eq!(got.as_deref(), Some("\x1bP0$r\x1b\\"), "{asked_for:?}");
    }
}

/// XTGETTCAP (`DCS + q`) is not DECRQSS: each scanner answers its own.
#[test]
fn xtgettcap_is_not_taken_for_decrqss() {
    let got = asked(b"\x1bP+q544e\x1b\\").unwrap_or_default();
    assert!(got.starts_with("\x1bP1+r"), "{got:?}");
    assert!(!got.contains("$r"), "{got:?}");
}

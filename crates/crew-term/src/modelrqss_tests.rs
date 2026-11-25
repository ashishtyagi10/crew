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

/// nvim's truecolor probe, set and ask in one write. nvim splits the reply
/// on `;` and accepts `0` plus ONE parameter matching `48:2:…` — xterm's
/// colon form, which is the only one it reads.
#[test]
fn nvim_reads_back_the_direct_background_it_just_set() {
    assert_eq!(
        asked(b"\x1b[48:2::1:2:3m\x1bP$qm\x1b\\"),
        ok("0;48:2::1:2:3m")
    );
    assert_eq!(
        asked(b"\x1b[48;2;1;2;3m\x1bP$qm\x1b\\"),
        ok("0;48:2::1:2:3m")
    );
}

/// xterm's `xtermFormatSGR`: attributes, then fg, then bg; the first sixteen
/// colours as `3x`/`9x`/`4x`/`10x` whether set by name or index, the rest
/// `:5:n`, direct `:2::r:g:b`.
#[test]
fn sgr_reports_attributes_and_colours_in_xterms_form() {
    assert_eq!(asked(b"\x1bP$qm\x1b\\"), ok("0m"));
    let got = asked(b"\x1b[1;2;3;9;31;48;5;208m\x1bP$qm\x1b\\");
    assert_eq!(got, ok("0;1;2;3;9;31;48:5:208m"));
    assert_eq!(
        asked(b"\x1b[7;8;94;101m\x1bP$qm\x1b\\"),
        ok("0;7;8;94;101m")
    );
    let indexed = asked(b"\x1b[38;5;3;48;5;12m\x1bP$qm\x1b\\");
    assert_eq!(
        indexed,
        ok("0;33;104m"),
        "the first sixteen fold as xterm folds them"
    );
    assert_eq!(
        asked(b"\x1b[38:2::9:8:7m\x1bP$qm\x1b\\"),
        ok("0;38:2::9:8:7m")
    );
    // Reported as crew parses it back (vte reads 21 as "bold off").
    assert_eq!(asked(b"\x1b[4:2m\x1bP$qm\x1b\\"), ok("0;4:2m"));
    assert_eq!(
        asked(b"\x1b[4:3;58;5;1m\x1bP$qm\x1b\\"),
        ok("0;4:3;58:5:1m")
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

/// Never styled: the steady block crew draws.
#[test]
fn an_unstyled_cursor_reports_the_steady_block() {
    assert_eq!(asked(b"\x1bP$q q\x1b\\"), ok("2 q"));
    assert_eq!(asked(b"\x1b[5 q\x1b[0 q\x1bP$q q\x1b\\"), ok("2 q"));
}

/// Inside a synchronized update the parser holds what came before the ask;
/// the answer waits for none of it to be lost — the held bytes are drawn,
/// and a cursor report asked first is still answered first.
#[test]
fn inside_a_synchronized_update_the_answer_follows_what_came_before() {
    let got = asked(b"\x1b[?2026h\x1b[31m\x1bP$qm\x1b\\\x1b[?2026l");
    assert_eq!(got, ok("0;31m"));
    let both = asked(b"\x1b[?2026h\x1b[6n\x1bP$qm\x1b\\\x1b[?2026l");
    assert_eq!(both, Some(format!("\x1b[1;1R{}", ok("0m").unwrap())));
}

/// A setting longer than any real one is still answered — unknown — not
/// left for the program to time out on.
#[test]
fn an_overlong_setting_is_answered_invalid() {
    let long = format!("\x1bP$q{}\x1b\\", "x".repeat(40));
    assert_eq!(asked(long.as_bytes()).as_deref(), Some("\x1bP0$r\x1b\\"));
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

/// A program saves the reply and replays it to restore: crew must read its
/// own answer back to the same state.
#[test]
fn a_replayed_sgr_reply_restores_the_same_state() {
    let set = b"\x1b[1;2;3;4:2;7;9;94;48:2::1:2:3;58:5:200m";
    let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 4 });
    t.feed(set);
    t.feed(b"\x1bP$qm\x1b\\");
    let first = t.take_replies().unwrap();
    let value = &first["\x1bP1$r".len()..first.len() - 2];
    t.feed(format!("\x1b[0m\x1b[{value}\x1bP$qm\x1b\\").as_bytes());
    assert_eq!(t.take_replies().unwrap(), first);
}

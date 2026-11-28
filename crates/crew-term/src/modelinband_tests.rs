//! DECSET 2048 on the model: the immediate report, DECRQM, and what clears it.
use crate::model::{GridSize, HeadlessTerm, TermModel};

fn term() -> HeadlessTerm {
    let mut t = HeadlessTerm::new(GridSize { cols: 80, rows: 24 });
    t.set_cell_px(9, 18);
    t
}

const REPORT: &str = "\x1b[48;24;80;432;720t";

#[test]
fn enabling_reports_the_current_size_at_once() {
    let mut t = term();
    t.feed(b"\x1b[?2048h");
    assert_eq!(t.take_replies().as_deref(), Some(REPORT));
    // Nothing more is owed until a resize.
    t.feed(b"hello");
    assert_eq!(t.take_replies(), None);
}

#[test]
fn enabling_split_across_reads_reports_once() {
    let mut t = term();
    for &b in b"\x1b[?20" {
        t.feed(&[b]);
    }
    assert_eq!(t.take_replies(), None);
    t.feed(b"48h");
    assert_eq!(t.take_replies().as_deref(), Some(REPORT));
}

#[test]
fn disabling_reports_nothing() {
    let mut t = term();
    t.feed(b"\x1b[?2048l");
    assert_eq!(t.take_replies(), None);
}

#[test]
fn decrqm_answers_set_and_reset() {
    let mut t = term();
    t.feed(b"\x1b[?2048$p");
    assert_eq!(t.take_replies().as_deref(), Some("\x1b[?2048;2$y"));
    t.feed(b"\x1b[?2048h");
    t.take_replies();
    t.feed(b"\x1b[?2048$p");
    assert_eq!(t.take_replies().as_deref(), Some("\x1b[?2048;1$y"));
    t.feed(b"\x1b[?2048l\x1b[?2048$p");
    assert_eq!(t.take_replies().as_deref(), Some("\x1b[?2048;2$y"));
}

#[test]
fn ris_and_decstr_clear_it() {
    for reset in [&b"\x1bc"[..], &b"\x1b[!p"[..]] {
        let mut t = term();
        t.feed(b"\x1b[?2048h");
        t.take_replies();
        t.feed(reset);
        t.feed(b"\x1b[?2048$p");
        assert_eq!(t.take_replies().as_deref(), Some("\x1b[?2048;2$y"));
    }
}

#[test]
fn it_shares_a_decset_with_other_modes() {
    let mut t = term();
    t.feed(b"\x1b[?1004;2048h");
    assert_eq!(t.take_replies().as_deref(), Some(REPORT));
}

#[test]
fn on_then_off_in_one_read_reports_nothing() {
    let mut t = term();
    t.feed(b"\x1b[?2048h\x1b[?2048l");
    assert_eq!(t.take_replies(), None);
    t.feed(b"\x1b[?2048h\x1bc");
    assert_eq!(t.take_replies(), None, "a reset in the same read too");
}

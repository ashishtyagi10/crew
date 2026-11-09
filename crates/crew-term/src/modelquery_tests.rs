//! XTVERSION through the whole terminal, as a program on the pty sees it.
use super::*;
use crate::model::{GridSize, HeadlessTerm, TermModel};

fn reply() -> String {
    format!("\x1bP>|crew {}\x1b\\", env!("CARGO_PKG_VERSION"))
}

#[test]
fn xtversion_names_crew_with_its_version() {
    for q in [&b"\x1b[>q"[..], b"\x1b[>0q"] {
        let mut t = HeadlessTerm::new(GridSize { cols: 80, rows: 24 });
        t.feed(q);
        assert_eq!(t.take_replies(), Some(reply()), "{q:?}");
    }
}

#[test]
fn xtversion_split_across_reads_still_answers() {
    let mut q = Queries::default();
    let mut out = String::new();
    for &b in b"x\x1b\x1b[>0q\x1b[" {
        out.push_str(&q.feed(&[b], (9, 18)));
    }
    assert_eq!(out, reply());
}

#[test]
fn near_misses_stay_silent() {
    let mut q = Queries::default();
    // DA2 (`> c`), XTMODKEYS (`> 4 ; 1 m`), a cursor-style set (`1 SP q`),
    // and an XTVERSION with a parameter that is not 0.
    assert_eq!(q.feed(b"\x1b[>c\x1b[>4;1m\x1b[1 q\x1b[>1q", (9, 18)), "");
}

#[test]
fn queries_answer_in_the_order_asked() {
    let mut q = Queries::default();
    let out = q.feed(b"\x1b[>q\x1b[16t", (9, 18));
    assert_eq!(out, format!("{}\x1b[6;18;9t", reply()));
}

/// DECRQM for 2031 through the whole terminal: crew's scanner says
/// "supported" and alacritty's "unknown" (`;0$y`) is not sent after it, or a
/// program reading the last answer turns its notifications off.
#[test]
fn decrqm_2031_is_answered_once() {
    let mut t = HeadlessTerm::new(GridSize { cols: 80, rows: 24 });
    t.feed(b"\x1b[?2031$p");
    assert_eq!(t.take_replies().as_deref(), Some("\x1b[?2031;2$y"));
    // Other modes alacritty doesn't know still get its honest answer.
    t.feed(b"\x1b[?2032$p");
    assert_eq!(t.take_replies().as_deref(), Some("\x1b[?2032;0$y"));
}

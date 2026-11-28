use super::*;
use crate::{GridSize, HeadlessTerm, TermModel};

#[test]
fn set_and_reset_track() {
    let mut p = PixelMouse::default();
    assert!(!p.on());
    p.feed(b"\x1b[?1016h");
    assert!(p.on());
    p.feed(b"\x1b[?1016l");
    assert!(!p.on());
}

#[test]
fn a_run_of_modes_and_split_reads_count() {
    let mut p = PixelMouse::default();
    p.feed(b"\x1b[?1000;1016;1006h");
    assert!(p.on());
    p.feed(b"\x1b[?1000;1016;1006l");
    assert!(!p.on());
    for &b in b"\x1b[?1016h" {
        p.feed(&[b]);
    }
    assert!(p.on(), "byte-at-a-time");
}

#[test]
fn near_misses_do_not_set_it() {
    let mut p = PixelMouse::default();
    p.feed(b"\x1b[?10160h\x1b[?101h\x1b[1016h\x1b[?1016;2h\x1b[?1016$h");
    assert!(p.on(), "only the `1016;2` run names it");
    let mut q = PixelMouse::default();
    q.feed(b"\x1b[?10160h\x1b[?101h\x1b[1016h\x1b[?1016$h");
    assert!(!q.on());
}

#[test]
fn decrqm_answers_set_and_reset() {
    let mut p = PixelMouse::default();
    assert_eq!(p.feed(b"\x1b[?1016$p"), "\x1b[?1016;2$y");
    p.feed(b"\x1b[?1016h");
    assert_eq!(p.feed(b"\x1b[?1016$p"), "\x1b[?1016;1$y");
    assert_eq!(p.feed(b"\x1b[?1006$p"), "", "other modes are not ours");
}

#[test]
fn ris_and_decstr_clear_it() {
    let mut p = PixelMouse::default();
    p.feed(b"\x1b[?1016h\x1bc");
    assert!(!p.on());
    p.feed(b"\x1b[?1016h\x1b[!p");
    assert!(!p.on());
}

#[test]
fn the_term_exposes_it_and_answers_decrqm_once() {
    let mut t = HeadlessTerm::new(GridSize { cols: 80, rows: 24 });
    t.feed(b"\x1b[?1000h\x1b[?1016h");
    let m = t.input_modes();
    assert!(m.mouse && m.pixel_mouse);
    t.feed(b"\x1b[?1016$p");
    // alacritty's own `;0$y` ("unknown") must not follow ours.
    assert_eq!(t.take_replies().as_deref(), Some("\x1b[?1016;1$y"));
    t.feed(b"\x1bc");
    assert!(!t.input_modes().pixel_mouse);
}

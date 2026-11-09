use super::super::{GridSize, HeadlessTerm, TermModel};

fn term() -> HeadlessTerm {
    HeadlessTerm::new(GridSize { cols: 20, rows: 4 })
}

/// xterm's reply form for `rgb`, as alacritty formats it.
fn rgb(osc: &str, (r, g, b): (u8, u8, u8)) -> String {
    format!("\x1b]{osc};rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}\x1b\\")
}

/// A saturated dark blue: no theme's echo-grey drop flattens it, so it is
/// drawn as set on every theme.
const BLUE: (u8, u8, u8) = (0x20, 0x40, 0x80);
const SET_BLUE: &[u8] = b"\x1b]11;rgb:20/40/80\x1b\\";

/// A program that sets a palette slot reads back what it set (base16-shell).
#[test]
fn osc4_reads_back_what_the_program_set() {
    let mut t = term();
    t.feed(b"\x1b]4;1;rgb:12/34/56\x1b\\\x1b]4;1;?\x1b\\");
    assert_eq!(t.take_replies(), Some(rgb("4;1", (0x12, 0x34, 0x56))));
}

/// The vim dance: set the background, ask for it, get it back; reset it
/// (OSC 111) and the theme's answer returns.
#[test]
fn osc11_reads_back_the_set_background_until_reset() {
    let mut t = term();
    t.feed(SET_BLUE);
    t.feed(b"\x1b]11;?\x1b\\");
    assert_eq!(t.take_replies(), Some(rgb("11", BLUE)));
    t.feed(b"\x1b]111\x1b\\\x1b]11;?\x1b\\");
    let theme = crew_theme::theme().term_bg;
    assert_eq!(t.take_replies(), Some(rgb("11", theme)));
}

/// A theme switch leaves a program's colour alone: crew never writes the
/// palette, and the one theme-dependent step — the echo-grey drop — keeps
/// this blue on a dark page and a light one alike. (Not switched for real:
/// the theme is a process global the other tests here read unguarded.)
#[test]
fn a_theme_switch_keeps_the_program_colour() {
    use super::super::modelcells::should_drop_bg;
    assert!(!should_drop_bg(BLUE, true, false) && !should_drop_bg(BLUE, false, false));
    let mut t = term();
    t.feed(SET_BLUE);
    t.feed(b"\x1b]11;?\x1b\\");
    t.feed(b"\x1b]11;?\x1b\\");
    assert_eq!(t.take_replies(), Some(rgb("11", BLUE).repeat(2)));
}

/// Black is an echo grey on every theme, so the pane draws its own
/// background over it — and says so, rather than naming a colour it
/// doesn't show.
#[test]
fn a_background_the_pane_does_not_draw_is_not_reported() {
    let mut t = term();
    t.feed(b"\x1b]11;rgb:00/00/00\x1b\\\x1b]11;?\x1b\\");
    let theme = crew_theme::theme().term_bg;
    assert_eq!(t.take_replies(), Some(rgb("11", theme)));
}

/// The cursor is drawn in the theme's colour, so OSC 12 keeps that answer.
#[test]
fn osc12_answers_the_drawn_cursor_not_the_set_one() {
    let mut t = term();
    t.feed(b"\x1b]12;rgb:20/40/80\x1b\\\x1b]12;?\x1b\\");
    assert_eq!(
        t.take_replies(),
        Some(rgb("12", crew_theme::theme().term_fg))
    );
}

/// Replies keep the order they were asked in: a colour query held for the
/// palette still lands before a cursor report asked after it.
#[test]
fn a_colour_reply_keeps_its_place_among_the_others() {
    let mut t = term();
    t.feed(b"\x1b[6n\x1b]4;2;?\x1b\\\x1b[6n");
    let green = crew_theme::theme().ansi[2];
    let want = format!("\x1b[1;1R{}\x1b[1;1R", rgb("4;2", green));
    assert_eq!(t.take_replies(), Some(want));
}

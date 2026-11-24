//! OSC 12 through the parser and onto the drawn cursor: what a program sets
//! is what the focused pane shows, floored so it never vanishes, and what a
//! query is told.
use crate::{GridSize, HeadlessTerm, RenderCell, TermModel};

/// A saturated orange: far from every theme's page, so it draws as set.
const ORANGE: (u8, u8, u8) = (0xe0, 0x70, 0x10);

fn cursor_cell(seq: &[u8], focused: bool) -> RenderCell {
    let mut t = HeadlessTerm::new(GridSize { cols: 10, rows: 3 });
    t.feed(seq);
    t.feed(b"ab");
    t.cells(focused)
        .into_iter()
        .find(|c| c.col == 2 && c.row == 0)
        .expect("a cell where the cursor is")
}

fn set(rgb: (u8, u8, u8)) -> Vec<u8> {
    let (r, g, b) = rgb;
    format!("\x1b]12;rgb:{r:02x}/{g:02x}/{b:02x}\x1b\\").into_bytes()
}

fn theme_block() -> (u8, u8, u8) {
    crew_theme::readable::cursor(crew_theme::theme(), true)
}

/// nvim's `guicursor` colour, a prompt's vi-mode colour: the block is it.
#[test]
fn the_focused_block_is_the_colour_the_program_set() {
    let page = crew_theme::theme().term_bg;
    assert!(crate::contrast::ratio(ORANGE, page) >= 3.0, "fixture");
    let cell = cursor_cell(&set(ORANGE), true);
    assert_eq!(cell.bg, ORANGE);
    assert_ne!(cell.bg, theme_block());
}

/// A bar or an underline is a mark, not a fill: the mark takes the colour.
#[test]
fn a_beam_is_ruled_in_the_set_colour() {
    let mut seq = set(ORANGE);
    seq.extend_from_slice(b"\x1b[6 q");
    let cell = cursor_cell(&seq, true);
    assert_eq!(cell.cursor.color, ORANGE);
}

/// The page's own colour as the cursor would hide where the keys go: it is
/// pushed to 3:1, WCAG's floor for a mark that is not text.
#[test]
fn a_cursor_set_to_the_page_colour_is_floored_until_it_shows() {
    let page = crew_theme::theme().term_bg;
    let cell = cursor_cell(&set(page), true);
    assert!(
        crate::contrast::ratio(cell.bg, page) >= 2.99,
        "{:?}",
        cell.bg
    );
}

/// OSC 112 hands the cursor back to the theme.
#[test]
fn osc112_resets_to_the_theme_cursor() {
    let mut seq = set(ORANGE);
    seq.extend_from_slice(b"\x1b]112\x1b\\");
    assert_eq!(cursor_cell(&seq, true).bg, theme_block());
}

/// An unfocused pane's outline says "not this pane" in every pane alike, so
/// a program's colour does not repaint it.
#[test]
fn an_unfocused_outline_keeps_the_theme_ink() {
    let plain = cursor_cell(b"", false).cursor.color;
    assert_eq!(cursor_cell(&set(ORANGE), false).cursor.color, plain);
}

/// Asked back, OSC 12 reports what the block is drawn in — the set colour,
/// or the floored one, or the theme's after a reset.
#[test]
fn osc12_answers_the_colour_the_block_is_drawn_in() {
    let page = crew_theme::theme().term_bg;
    for (seq, want) in [
        (set(ORANGE), ORANGE),
        (set(page), cursor_cell(&set(page), true).bg),
        (b"\x1b]112\x1b\\".to_vec(), theme_block()),
    ] {
        let mut t = HeadlessTerm::new(GridSize { cols: 10, rows: 3 });
        t.feed(&seq);
        t.take_replies();
        t.feed(b"\x1b]12;?\x1b\\");
        let (r, g, b) = want;
        let reply = format!("\x1b]12;rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}\x1b\\");
        assert_eq!(t.take_replies(), Some(reply));
    }
}

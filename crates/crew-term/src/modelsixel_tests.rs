//! A sixel through a live terminal: where it lands, where the cursor ends up,
//! and how the terminal says it can draw one.
use crate::model::{GridSize, HeadlessTerm, TermModel};

fn term() -> HeadlessTerm {
    let mut t = HeadlessTerm::new(GridSize { cols: 40, rows: 12 });
    t.set_cell_px(10, 20);
    t
}

/// 20 x 60 pixels: two cells wide, three rows tall at 10 x 20, all drawn
/// (ten bands).
const SIXEL: &str = "\x1bPq\"1;1;20;60#1!20~-!20~-!20~-!20~-!20~-!20~-!20~-!20~-!20~-!20~\x1b\\";

/// The cursor after a sixel is on the picture's LAST row, in the column the
/// picture started at (foot's `sixel_unhook`), asked the way a program asks.
#[test]
fn the_cursor_ends_on_the_last_row_at_the_start_column() {
    let mut t = term();
    t.feed(format!("\r\nab{SIXEL}\x1b[6n").as_bytes());
    let imgs = t.take_images();
    assert_eq!(imgs.len(), 1);
    assert!(imgs[0].cmd.sixel);
    assert_eq!((imgs[0].line, imgs[0].col, imgs[0].cells), (1, 2, (2, 3)));
    // Started at row 2 col 3 (1-based); three rows tall means row 4, col 3.
    assert_eq!(t.take_replies().as_deref(), Some("\x1b[4;3R"));
}

#[test]
fn a_sixel_is_never_answered_and_never_printed() {
    let mut t = term();
    t.feed(format!("x{SIXEL}y").as_bytes());
    assert_eq!(t.take_replies(), None, "no id, so no OK");
    let text = t.cells(false).iter().map(|c| c.c).collect::<String>();
    assert!(text.contains('x') && text.contains('y'));
    assert!(!text.contains('~') && !text.contains('#'), "{text:?}");
}

#[test]
fn a_sixel_taller_than_the_screen_scrolls_the_grid() {
    let mut t = term();
    t.feed(b"\x1b[12;1H");
    t.feed(SIXEL.as_bytes());
    t.feed(b"\x1b[6n");
    assert_eq!(t.take_replies().as_deref(), Some("\x1b[12;1R"));
    assert_eq!(t.history_lines(), 2, "two lines scrolled to make room");
}

#[test]
fn da1_claims_sixel_and_ansi_colour_and_nothing_else() {
    for q in ["\x1b[c", "\x1b[0c"] {
        let mut t = term();
        t.feed(q.as_bytes());
        assert_eq!(t.take_replies().as_deref(), Some("\x1b[?62;4;22c"), "{q:?}");
    }
}

#[test]
fn the_colour_register_count_is_answered() {
    for q in ["\x1b[?1;1;0S", "\x1b[?1;4;0S"] {
        let mut t = term();
        t.feed(q.as_bytes());
        assert_eq!(t.take_replies().as_deref(), Some("\x1b[?1;0;256S"));
    }
    let mut t = term();
    t.feed(b"\x1b[?2;1;0S");
    assert_eq!(t.take_replies(), None, "geometry is a documented gap");
}

/// The cursor follows what the data draws; the raster only reserves rows
/// (foot: raster attributes "do *not* affect cursor positioning"). One band
/// under a 60 px raster is one text row, but all three rows are made room.
#[test]
fn a_raster_taller_than_the_data_reserves_rows_but_the_cursor_follows_the_data() {
    let mut t = term();
    t.feed(b"\x1b[11;1H\x1bPq\"1;1;20;60#1!20~\x1b\\\x1b[6n");
    let imgs = t.take_images();
    assert_eq!(imgs[0].cells, (2, 3), "the raster reserves three rows");
    // Row 11 + 2 line feeds scrolled the bottom (12 rows): the picture's
    // first row is 10, and the cursor stays on it.
    assert_eq!(t.take_replies().as_deref(), Some("\x1b[10;1R"));
    assert_eq!(
        t.history_lines(),
        1,
        "the grid still scrolled for the picture"
    );
}

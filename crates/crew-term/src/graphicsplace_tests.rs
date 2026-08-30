//! A picture arriving in a live terminal: where it lands, what it costs the
//! grid, and what must never appear on screen.
use super::*;
use crate::model::{GridSize, HeadlessTerm};

fn png(w: u32, h: u32) -> Vec<u8> {
    // Only the header is read before a picture is placed, and only the header
    // is needed to say how many rows it claims.
    let mut v = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR".to_vec();
    v.extend(w.to_be_bytes());
    v.extend(h.to_be_bytes());
    v.extend([8, 6, 0, 0, 0]);
    v
}

fn seq(keys: &str, payload: &[u8]) -> String {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(payload);
    format!("\x1b_G{keys};{b64}\x1b\\")
}

fn term() -> HeadlessTerm {
    let mut t = HeadlessTerm::new(GridSize { cols: 40, rows: 12 });
    t.set_cell_px(10, 20);
    t
}

/// The row the picture goes on is where the cursor was when it arrived — so
/// the bytes before the sequence have to reach the parser first.
#[test]
fn a_picture_lands_where_the_cursor_was_not_where_the_chunk_ended() {
    let mut t = term();
    let s = format!("one\r\ntwo\r\n{}three\r\n", seq("a=T,f=100", &png(100, 40)));
    t.feed(s.as_bytes());
    let imgs = t.take_images();
    assert_eq!(imgs.len(), 1);
    assert_eq!(imgs[0].line, 2, "the third line, where the cursor was");
    assert_eq!(imgs[0].col, 0);
}

/// A picture with no `c`/`r` claims the cells its pixels need — which is why
/// the app publishes the cell size.
#[test]
fn its_size_comes_from_its_pixels_over_the_cell() {
    let mut t = term();
    t.feed(seq("a=T,f=100", &png(100, 40)).as_bytes());
    assert_eq!(t.take_images()[0].cells, (10, 2), "100/10 by 40/20");
    // …and an explicit request outranks that.
    let mut t = term();
    t.feed(seq("a=T,f=100,c=6,r=3", &png(100, 40)).as_bytes());
    assert_eq!(t.take_images()[0].cells, (6, 3));
}

/// The room a picture takes has to be real room: the terminal scrolls past it
/// so the next line of output does not print on top of it.
#[test]
fn the_terminal_moves_past_the_picture_it_placed() {
    let mut t = term();
    t.feed(format!("{}after\r\n", seq("a=T,f=100,c=8,r=4", &png(80, 80))).as_bytes());
    let rows: Vec<u16> = t
        .cells(true)
        .iter()
        .filter(|c| c.c == 'a')
        .map(|c| c.row)
        .collect();
    assert_eq!(
        rows,
        vec![4],
        "the text after it starts below its four rows"
    );
}

/// Not one byte of a picture may reach the screen. A terminal that prints the
/// base64 is worse than one that ignores the sequence entirely.
#[test]
fn none_of_the_sequence_is_ever_printed() {
    let mut t = term();
    t.feed(seq("a=T,f=100", &png(100, 40)).as_bytes());
    let text: String = t.cells(true).iter().map(|c| c.c).collect();
    assert!(
        !text.contains('G') && !text.contains('='),
        "escape text on screen: {text:?}"
    );
}

/// `a=d` is the program taking its pictures back.
#[test]
fn a_delete_clears_what_is_on_screen() {
    let mut t = term();
    t.feed(seq("a=T,f=100", &png(20, 20)).as_bytes());
    t.feed(seq("a=d", b"").as_bytes());
    assert!(t.take_images().is_empty());
}

/// A transmit-only command stores for a later placement crew does not keep a
/// store for — it must not draw, and must not scroll the screen either.
#[test]
fn a_transmit_without_a_display_changes_nothing() {
    let mut t = term();
    t.feed(format!("{}x", seq("a=t,f=100", &png(100, 100))).as_bytes());
    assert!(t.take_images().is_empty());
    let x = t.cells(true).iter().find(|c| c.c == 'x').map(|c| c.row);
    assert_eq!(x, Some(0), "the screen did not move");
}

/// Image tools ask before they send. A terminal that never answers is one
/// every one of them decides cannot draw a picture.
#[test]
fn a_capability_probe_is_answered() {
    let mut t = term();
    t.feed(seq("i=31,s=1,v=1,a=q,t=d,f=24", &[0, 0, 0]).as_bytes());
    assert_eq!(t.take_replies().as_deref(), Some("\x1b_Gi=31;OK\x1b\\"));
    assert!(t.take_images().is_empty(), "a probe places nothing");
}

/// …and a format crew cannot decode is refused, rather than accepted and then
/// quietly dropped — the producer can fall back to something else.
#[test]
fn a_probe_for_a_format_crew_cannot_draw_is_refused() {
    let mut t = term();
    t.feed(seq("i=7,a=q,f=1000", b"x").as_bytes());
    let reply = t.take_replies().unwrap_or_default();
    assert!(reply.contains("ENOTSUPPORTED"), "answered {reply:?}");
}

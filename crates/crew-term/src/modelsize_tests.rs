//! The size reports a program reads: `CSI 14 t`, `CSI 16 t` and the pty's
//! pixel fields, all from the grid and the cell size the app published.
use super::*;
use crate::model::{GridSize, HeadlessTerm, TermModel};

fn term() -> HeadlessTerm {
    let mut t = HeadlessTerm::new(GridSize { cols: 80, rows: 24 });
    t.set_cell_px(9, 18);
    t
}

#[test]
fn csi_14t_reports_the_text_area_in_pixels() {
    let mut t = term();
    t.feed(b"\x1b[14t");
    assert_eq!(t.take_replies().as_deref(), Some("\x1b[4;432;720t"));
}

#[test]
fn csi_16t_reports_one_cell_in_pixels() {
    let mut t = term();
    t.feed(b"\x1b[16t");
    assert_eq!(t.take_replies().as_deref(), Some("\x1b[6;18;9t"));
}

#[test]
fn the_reports_follow_a_resize_and_a_new_cell_size() {
    let mut t = term();
    t.resize(GridSize { cols: 40, rows: 10 });
    t.set_cell_px(10, 20);
    t.feed(b"\x1b[14t\x1b[16t");
    assert_eq!(
        t.take_replies().as_deref(),
        Some("\x1b[6;20;10t\x1b[4;200;400t"),
        "16t is answered by the scanner ahead of the parser's 14t"
    );
}

#[test]
fn a_cell_query_split_across_reads_still_answers() {
    let mut q = Queries::default();
    let mut out = String::new();
    for &b in b"ab\x1b\x1b[16t\x1b[1" {
        out.push_str(&q.feed(&[b], (9, 18)));
    }
    assert_eq!(out, "\x1b[6;18;9t");
    // Not ours: other `t` reports and longer parameter runs stay silent.
    assert_eq!(q.feed(b"\x1b[18t\x1b[116t\x1b[16;1t", (9, 18)), "");
}

#[test]
fn the_pty_size_carries_the_text_area_in_pixels() {
    let mut t = TermCore::new(GridSize { cols: 80, rows: 24 });
    t.set_cell_px(9, 18);
    t.resize(GridSize {
        cols: 100,
        rows: 30,
    });
    let s = t.pty_size();
    assert_eq!((s.cols, s.rows), (100, 30));
    assert_eq!((s.pixel_width, s.pixel_height), (900, 540));
}

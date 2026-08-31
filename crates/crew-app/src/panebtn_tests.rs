use super::{btn_color, draw, GLYPHS};
use crate::panehover::Btn;
use crew_render::CellView;

/// Deliberately not a theme colour, so "still at rest" is unambiguous.
const LEGEND: (u8, u8, u8) = (10, 20, 30);

fn cells(hover: Option<Btn>) -> Vec<CellView> {
    let mut v = Vec::new();
    draw(&mut v, 40, LEGEND, hover);
    v
}

#[test]
fn a_resting_pair_is_the_legend_colour_throughout() {
    let v = cells(None);
    assert_eq!(v.len(), GLYPHS.chars().count());
    assert!(v.iter().all(|c| c.fg == LEGEND && !c.bold));
    let drawn: String = v.iter().map(|c| c.c).collect();
    assert_eq!(drawn, GLYPHS);
}

#[test]
fn hovering_one_button_leaves_the_other_at_rest() {
    let v = cells(Some(Btn::Min));
    let (lit, rest) = v.split_at(3);
    assert!(lit.iter().all(|c| c.fg != LEGEND && c.bold), "[-] lights");
    assert!(
        rest.iter().all(|c| c.fg == LEGEND && !c.bold),
        "[x] stays at rest"
    );
}

#[test]
fn close_and_minimize_light_in_different_colours() {
    let _g = crate::app::theme_test_guard();
    // `[x]` ends a running shell; it must not read like `[-]`, which does
    // not. Same hover, deliberately different answer.
    assert_ne!(
        btn_color(Btn::Close, true, LEGEND),
        btn_color(Btn::Min, true, LEGEND)
    );
}

#[test]
fn the_pair_sits_where_the_hit_rects_look_for_it() {
    // `close_btn_rect` reads columns `cols-5 ..= cols-3` and
    // `min_btn_rect` `cols-8 ..= cols-6`; draw must agree exactly, or the
    // lit glyph and the clickable region drift apart.
    let cols = 40u16;
    let v = cells(None);
    let at = |i: usize| v[i].col;
    assert_eq!((at(0), at(2)), (cols - 8, cols - 6), "[-] columns");
    assert_eq!((at(3), at(5)), (cols - 5, cols - 3), "[x] columns");
}

//! The `[-]` `[x]` buttons on a pane card's top border: how they are drawn,
//! and how they answer the pointer.
//!
//! Split out of [`crate::panecard`] (which was over the file cap) so the one
//! piece of the frame that is a *control* rather than decoration has a home.
//! Hit rects stay in `panecard` beside `card_inner_cells`, because draw and
//! hit must share that one convention.
use crew_render::CellView;

use crate::panehover::Btn;

/// The glyph pair, in the order they sit on the border.
const GLYPHS: &str = "[-][x]";

/// Where the pair starts, counted back from the card's right edge.
const FROM_RIGHT: u16 = 8;

/// The colour a button wears. Resting, it recedes into the legend it shares a
/// border with — the frame stays a frame. Under the pointer it takes the
/// accent, except `[x]`, which takes the bell colour: the one control on the
/// canvas that ends a running shell should say so before it is clicked, not
/// after.
fn btn_color(which: Btn, hovered: bool, legend: (u8, u8, u8)) -> (u8, u8, u8) {
    match (hovered, which) {
        (false, _) => legend,
        (true, Btn::Min) => crate::palette::accent(),
        (true, Btn::Close) => crew_theme::theme().bell,
    }
}

/// Stamp `[-][x]` onto the already-drawn top border of a `cols`-wide card,
/// lighting whichever button `hover` names. Writes through
/// [`crate::panecard::put`], so the border keeps one way of overwriting a
/// cell on row 0.
pub(crate) fn draw(v: &mut Vec<CellView>, cols: u16, legend: (u8, u8, u8), hover: Option<Btn>) {
    use crate::panecard::put;
    for (i, ch) in GLYPHS.chars().enumerate() {
        // The first three glyphs are `[-]`, the last three `[x]`.
        let which = if i < 3 { Btn::Min } else { Btn::Close };
        let lit = hover == Some(which);
        let col = cols - FROM_RIGHT + i as u16;
        put(v, col, 0, ch, btn_color(which, lit, legend), lit);
    }
}

#[cfg(test)]
mod tests {
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
}

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
#[path = "panebtn_tests.rs"]
mod tests;

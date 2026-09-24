//! The status badges on a card's top border, seated the way its legend is.
//!
//! The legend reads ` 2 crew · claude ` — a space either side, a `·` between
//! its parts. The badges on the right of the same rule were stamped straight
//! onto the stroke one cell apart, so a busy card read `main─●3─1m02s─8─●`:
//! the rule ran through the middle of every reading. This runs after they
//! are all down and gives them the legend's grammar: the cell of rule between
//! two badges becomes a `·` (the nav's `1·5·15m`), and a space parts the
//! cluster from the rule on either side.
use crew_render::CellView;

use crate::panecard::{is_frame_glyph, put};

/// Seat every badge stamped on row 0 between `floor` and `last` (the last
/// column a badge may use — left of the corner buttons).
pub(crate) fn seat(v: &mut Vec<CellView>, floor: u16, last: u16, dot: (u8, u8, u8)) {
    let at = |v: &[CellView], col: u16| v.iter().find(|c| c.row == 0 && c.col == col).map(|c| c.c);
    let text: Vec<u16> = v
        .iter()
        .filter(|c| c.row == 0 && (floor..=last).contains(&c.col))
        .filter(|c| c.c != ' ' && !is_frame_glyph(c.c))
        .map(|c| c.col)
        .collect();
    let (Some(&lo), Some(&hi)) = (text.iter().min(), text.iter().max()) else {
        return;
    };
    for col in lo..=hi {
        if at(v, col).is_none_or(is_frame_glyph) {
            put(v, col, 0, '\u{b7}', dot, false);
        }
    }
    for col in [lo.saturating_sub(1), hi + 1] {
        if col > 0 && at(v, col).is_none_or(is_frame_glyph) {
            put(v, col, 0, ' ', dot, false);
        }
    }
}

#[cfg(test)]
#[path = "borderseat_tests.rs"]
mod tests;

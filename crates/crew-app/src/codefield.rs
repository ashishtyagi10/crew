//! The code field's colour: how far a fenced block's background sits from
//! the page. Split from [`crate::chatink`] (line cap) when light pages got
//! their own, quieter floor.
use crate::chatbody::Color;
use crate::chatink::{CODE_BG_MIX, CODE_ON_FIELD_FLOOR, FIELD_FLOOR};
use crew_theme::{contrast_ratio, Theme};

/// The floor on a LIGHT page. 1.55 is a tube's number: on paper it walked the
/// field a third of the way to the ink, a mid-grey slab the comments and
/// strings went muddy on. Paper has no bloom to eat a difference, so a code
/// block there is a step off the page — the selection wash's lesson.
const LIGHT_FIELD_FLOOR: f32 = 1.2;

/// Where a light page's field starts walking from, below `CODE_BG_MIX`.
const LIGHT_BG_MIX: f32 = 0.04;

/// The floor the field clears on `t`'s page.
pub(crate) fn field_floor(t: &Theme) -> f32 {
    if t.dark {
        FIELD_FLOOR
    } else {
        LIGHT_FIELD_FLOOR
    }
}

/// The code field's background: the page walked toward `ink` until it clears
/// [`field_floor`], stopping early if `code` would stop reading on it.
pub(crate) fn code_field(t: &Theme, code: Color) -> Color {
    let mut mix = if t.dark { CODE_BG_MIX } else { LIGHT_BG_MIX };
    let mut best = crate::anim::lerp_rgb(t.page_bg, t.ink, mix);
    while mix < 1.0 && contrast_ratio(best, t.page_bg) < field_floor(t) {
        mix += 0.01;
        let cand = crate::anim::lerp_rgb(t.page_bg, t.ink, mix);
        if contrast_ratio(code, cand) < CODE_ON_FIELD_FLOOR {
            break;
        }
        best = cand;
    }
    best
}

#[cfg(test)]
#[path = "codefield_tests.rs"]
mod tests;

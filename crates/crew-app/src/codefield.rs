//! The code field's colour: how far a fenced block's background sits from
//! the page. Split from [`crate::chatink`] (line cap) when light pages got
//! their own, quieter floor.
use crate::chatbody::Color;
use crate::chatink::{
    CODE_BG_MIX, CODE_ON_FIELD_FLOOR, COMMENT_FLOOR, COMMENT_PAGE_FLOOR, FIELD_FLOOR,
};
use crew_theme::{contrast_ratio, Theme};

/// The floor on a LIGHT page. 1.55 is a tube's number: on paper it walked the
/// field a third of the way to the ink, a mid-grey slab the comments and
/// strings went muddy on. Paper has no bloom to eat a difference, so a code
/// block there is a step off the page — the selection wash's lesson.
const LIGHT_FIELD_FLOOR: f32 = 1.2;

/// The floor on a TUBE. A step under the dark pages' [`FIELD_FLOOR`]: on a
/// one-hue screen code, comments and the field are one phosphor at three
/// lightnesses, and at 1.55 the field left a comment 2.7:1 on it — the room
/// the comment needs (`chatink::derive`) is what the field gives back. The
/// tube's glass no longer carries scanlines to eat a faint field.
const TUBE_FIELD_FLOOR: f32 = 1.45;

/// Where a light page's field starts walking from, below `CODE_BG_MIX`.
const LIGHT_BG_MIX: f32 = 0.04;

/// The floor the field clears on `t`'s page.
pub(crate) fn field_floor(t: &Theme) -> f32 {
    match (t.is_tube(), t.dark) {
        (true, _) => TUBE_FIELD_FLOOR,
        (false, true) => FIELD_FLOOR,
        (false, false) => LIGHT_FIELD_FLOOR,
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

/// How readable a comment is held on the code field — where it is drawn, a
/// step off the page toward the ink: floored against the PAGE it measured
/// 4.2:1 there and 2.7:1 where it stands (every tube; 3.3 on paper-dark).
/// [`COMMENT_PAGE_FLOOR`],
/// or on a tube as much of it as leaves the comment a rung (`TUBE_RUNG`)
/// under code — past that, comment and code are one shade of one phosphor.
pub(crate) fn comment_room(t: &Theme, code: Color, field: Color) -> f32 {
    const TUBE_RUNG: f32 = 1.62;
    match t.is_tube() {
        true => COMMENT_PAGE_FLOOR.min(contrast_ratio(code, field) / TUBE_RUNG),
        false => COMMENT_PAGE_FLOOR,
    }
}

/// A comment's colour: `text_muted` walked back from the ink (the ladder's
/// furthest rung) while it still reads [`comment_room`] on the field.
pub(crate) fn comment(t: &Theme, code: Color, field: Color) -> Color {
    let room = comment_room(t, code, field);
    separated_on(t.text_muted, t, COMMENT_FLOOR, room, field)
}

/// `chatink::separated_to` for a colour drawn on `ground` rather than the page: the
/// walk stops before `c` reads under `ground_floor` on what is actually
/// behind it.
pub(crate) fn separated_on(
    c: Color,
    t: &Theme,
    floor: f32,
    ground_floor: f32,
    ground: Color,
) -> Color {
    if contrast_ratio(c, t.ink) >= floor {
        return c;
    }
    let mut best = c;
    let mut mix = 0.0_f32;
    while mix < 1.0 {
        mix += 0.005;
        let cand = crate::anim::lerp_rgb(c, t.page_bg, mix);
        // Stop before readability goes: a preset with no room to separate
        // keeps the last legible candidate rather than fading into the page.
        if contrast_ratio(cand, ground) < ground_floor {
            break;
        }
        best = cand;
        if contrast_ratio(cand, t.ink) >= floor {
            break;
        }
    }
    best
}

#[cfg(test)]
#[path = "codefield_tests.rs"]
mod tests;

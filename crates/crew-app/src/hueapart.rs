//! Apart from prose by hue: the door a coloured syntax class takes past the
//! luminance walk ([`crate::codefield::separated_on`]). Split from
//! `codefield` (line cap).
use crate::chatbody::Color;
use crew_theme::{oklch, Theme};

/// How far apart in hue and chroma (OKLab's a-b plane) a colour must sit
/// from the ink to count as apart from prose without a step of lightness.
/// Measured over all 20 palettes: every hued class (code, strings, markers)
/// sits 0.074 or more from its ink, every grey one (muted text, the
/// classics' comment tints) 0.049 or less.
const HUE_APART: f32 = 0.06;

/// `chatink::separated_to`: a colour of a different hue than the ink is
/// already apart from prose and is drawn at its own lightness; a grey one,
/// or anything on a single-phosphor tube, is walked toward the page as
/// before. The walk was a luminance ratio, which cannot see hue: on every
/// dark page it dimmed cyan code to APCA Lc 53 on its field and strings to
/// 42, where the light pages, whose ink is far from any hue already, kept
/// theirs at 62-67. Comments never take this door: they are the class
/// meant to recede ([`crate::codefield::comment`]).
pub(crate) fn separated_or_hued(c: Color, t: &Theme, floor: f32, page_floor: f32) -> Color {
    if hued_apart(c, t) {
        return c;
    }
    crate::codefield::separated_on(c, t, floor, page_floor, t.page_bg)
}

/// Whether `c` reads apart from `t`'s prose by hue alone — never on a tube,
/// whose classes share the ink's one phosphor.
pub(crate) fn hued_apart(c: Color, t: &Theme) -> bool {
    !t.is_tube() && hue_apart(c, t.ink) >= HUE_APART
}

/// The distance between `a` and `b` in OKLab's a-b plane: hue and chroma,
/// with lightness left out.
pub(crate) fn hue_apart(a: Color, b: Color) -> f32 {
    let ab = |c: Color| {
        let o = oklch::from_srgb(c);
        let h = o.h.to_radians();
        (o.c * h.cos(), o.c * h.sin())
    };
    let ((a1, b1), (a2, b2)) = (ab(a), ab(b));
    (a1 - a2).hypot(b1 - b2)
}

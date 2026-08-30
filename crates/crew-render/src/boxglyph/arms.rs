//! The line characters, as four arms out of one junction.
//!
//! Every character in U+2500–U+254B is the same drawing with different arms:
//! `─` is left+right, `┌` is right+down, `┼` is all four, and each arm is
//! either absent, light, or heavy. Tabulating the arms is far less code than
//! tabulating the rectangles, and it is also what makes the set *join*: the
//! two strokes are cut from the same centred span, so a `├` meets the `│`
//! above it exactly, in every cell, at every size.
use super::{centre, light_thickness, Mask};

/// Arm weights, clockwise from the top: `0` none, `1` light, `2` heavy.
type Arms = [u8; 4];

/// `(char, [up, right, down, left])` for the light/heavy box-drawing set.
/// The dashed (`┄┈┆┊`) and double (`═║╔`) runs are deliberately absent —
/// crew draws with none of them, and a character this table does not claim
/// falls through to the font exactly as before.
const TABLE: &[(char, Arms)] = &[
    ('\u{2500}', [0, 1, 0, 1]),
    ('\u{2501}', [0, 2, 0, 2]),
    ('\u{2502}', [1, 0, 1, 0]),
    ('\u{2503}', [2, 0, 2, 0]),
    ('\u{250C}', [0, 1, 1, 0]),
    ('\u{250D}', [0, 2, 1, 0]),
    ('\u{250E}', [0, 1, 2, 0]),
    ('\u{250F}', [0, 2, 2, 0]),
    ('\u{2510}', [0, 0, 1, 1]),
    ('\u{2511}', [0, 0, 1, 2]),
    ('\u{2512}', [0, 0, 2, 1]),
    ('\u{2513}', [0, 0, 2, 2]),
    ('\u{2514}', [1, 1, 0, 0]),
    ('\u{2515}', [1, 2, 0, 0]),
    ('\u{2516}', [2, 1, 0, 0]),
    ('\u{2517}', [2, 2, 0, 0]),
    ('\u{2518}', [1, 0, 0, 1]),
    ('\u{2519}', [1, 0, 0, 2]),
    ('\u{251A}', [2, 0, 0, 1]),
    ('\u{251B}', [2, 0, 0, 2]),
    ('\u{251C}', [1, 1, 1, 0]),
    ('\u{2520}', [2, 1, 2, 0]),
    ('\u{2523}', [2, 2, 2, 0]),
    ('\u{2524}', [1, 0, 1, 1]),
    ('\u{2528}', [2, 0, 2, 1]),
    ('\u{252B}', [2, 0, 2, 2]),
    ('\u{252C}', [0, 1, 1, 1]),
    ('\u{2530}', [0, 1, 2, 1]),
    ('\u{2533}', [0, 2, 2, 2]),
    ('\u{2534}', [1, 1, 0, 1]),
    ('\u{2538}', [2, 1, 0, 1]),
    ('\u{253B}', [2, 2, 0, 2]),
    ('\u{253C}', [1, 1, 1, 1]),
    ('\u{2542}', [2, 1, 2, 1]),
    ('\u{254B}', [2, 2, 2, 2]),
];

/// `(char, dashes, heavy, vertical)` for the dashed runs — U+2504–250B
/// (three and four dashes) and U+254C–254F (two). A dashed rule is the same
/// stroke as its solid sibling with the ink taken out in even steps, and it
/// wants the same pixel snapping for the same reason: a dash that lands
/// between two pixels is two grey dashes.
const DASHED: &[(char, u32, bool, bool)] = &[
    ('\u{2504}', 3, false, false),
    ('\u{2505}', 3, true, false),
    ('\u{2506}', 3, false, true),
    ('\u{2507}', 3, true, true),
    ('\u{2508}', 4, false, false),
    ('\u{2509}', 4, true, false),
    ('\u{250A}', 4, false, true),
    ('\u{250B}', 4, true, true),
    ('\u{254C}', 2, false, false),
    ('\u{254D}', 2, true, false),
    ('\u{254E}', 2, false, true),
    ('\u{254F}', 2, true, true),
];

/// Draw `c` if it is a dashed rule: `n` marks evenly spaced along the cell,
/// each two thirds of its step, so the gaps read at a terminal's size
/// without the rule breaking up into dots.
fn dashed(m: &mut Mask, c: char) -> bool {
    let Some(&(_, n, heavy, vertical)) = DASHED.iter().find(|(k, ..)| *k == c) else {
        return false;
    };
    let light = light_thickness(m.h);
    let t = if heavy { light * 2 } else { light };
    let extent = if vertical { m.h } else { m.w };
    let step = extent / n;
    let mark = (step * 2 / 3).max(1);
    let (a0, a1) = centre(if vertical { m.w } else { m.h }, t);
    for i in 0..n {
        let lo = (i * step) as f32;
        let hi = lo + mark as f32;
        if vertical {
            m.rect(a0 as f32, lo, a1 as f32, hi);
        } else {
            m.rect(lo, a0 as f32, hi, a1 as f32);
        }
    }
    true
}

/// The arms of `c`, if this module draws it.
pub(super) fn arms_of(c: char) -> Option<Arms> {
    TABLE.iter().find(|(k, _)| *k == c).map(|(_, a)| *a)
}

/// A stroke of `weight` across `extent`: its `[lo, hi)` pixel span, centred.
/// A heavy stroke is twice a light one, which is the ratio the box-drawing
/// block was designed at and the only one that keeps `┃` visibly heavier
/// than `│` at a 1px light thickness.
pub(super) fn span(extent: u32, weight: u8, light: u32) -> (u32, u32) {
    centre(extent, if weight >= 2 { light * 2 } else { light })
}

/// Draw `c` if it is a line character. The horizontal and vertical strokes
/// are cut from the widest arm on each axis, so `┝` (light vertical, heavy
/// right) keeps each stroke its own weight while still meeting cleanly.
pub(super) fn draw(m: &mut Mask, c: char) -> bool {
    if dashed(m, c) {
        return true;
    }
    let Some(a) = arms_of(c) else {
        return false;
    };
    let light = light_thickness(m.h);
    let (cw, ch) = (m.w as f32, m.h as f32);
    // The vertical stroke's column span, and the horizontal stroke's row span.
    let vw = a[0].max(a[2]);
    let hw = a[1].max(a[3]);
    let (vx0, vx1) = span(m.w, vw, light);
    let (hy0, hy1) = span(m.h, hw, light);
    // Arms stop at the FAR edge of the perpendicular stroke, so a corner is
    // filled through its junction rather than leaving a notch at the turn.
    if a[0] > 0 {
        m.rect(vx0 as f32, 0.0, vx1 as f32, hy1.max(hy0 + 1) as f32);
    }
    if a[2] > 0 {
        m.rect(
            vx0 as f32,
            hy0.min(hy1.saturating_sub(1)) as f32,
            vx1 as f32,
            ch,
        );
    }
    if a[3] > 0 {
        m.rect(0.0, hy0 as f32, vx1.max(vx0 + 1) as f32, hy1 as f32);
    }
    if a[1] > 0 {
        m.rect(
            vx0.min(vx1.saturating_sub(1)) as f32,
            hy0 as f32,
            cw,
            hy1 as f32,
        );
    }
    true
}

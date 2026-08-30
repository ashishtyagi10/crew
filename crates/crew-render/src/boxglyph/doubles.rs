//! The double-line set, U+2550–U+256C.
//!
//! Not crew's own furniture — crew frames with the light set — but the set a
//! great many of the programs that live in its panes frame with: lazygit,
//! ncdu, midnight commander, half the ncurses dialogs ever written. A pane
//! running one of those was drawing its whole frame from the font, dilated
//! and curved like a letter, which is the softness this module exists to end
//! and there is no reason it should stop at crew's own borders.
//!
//! A double line is two light strokes with a light gap between them, so the
//! band is three thicknesses across. What makes the set hard is the corners:
//! at a turn the OUTER stroke of each arm runs to the outer edge of the
//! other's band and the INNER stroke to the inner edge, or the corner comes
//! out as a lattice with its corner missing.
use super::{centre, light_thickness, Mask};

/// Arms present, clockwise from the top.
type Arms = [bool; 4];

/// `(char, [up, right, down, left])` for U+2550–U+256C. The mixed
/// single/double junctions (`╞ ╤ ╫` …) are left to the font: they are rare,
/// and each is its own asymmetric special case rather than a member of this
/// family.
const TABLE: &[(char, Arms)] = &[
    ('\u{2550}', [false, true, false, true]),
    ('\u{2551}', [true, false, true, false]),
    ('\u{2554}', [false, true, true, false]),
    ('\u{2557}', [false, false, true, true]),
    ('\u{255A}', [true, true, false, false]),
    ('\u{255D}', [true, false, false, true]),
    ('\u{2560}', [true, true, true, false]),
    ('\u{2563}', [true, false, true, true]),
    ('\u{2566}', [false, true, true, true]),
    ('\u{2569}', [true, true, false, true]),
    ('\u{256C}', [true, true, true, true]),
];

/// The two parallel strokes of a double band across `extent`: `[a, a+t)` and
/// `[b, b+t)`, with one thickness of gap between them.
fn pair(extent: u32, t: u32) -> (u32, u32) {
    let (lo, _) = centre(extent, 3 * t);
    (lo, lo + 2 * t)
}

/// One stroke's extent along its axis, minus the gap where a perpendicular
/// pair passes through it. Two rectangles when the gap falls inside, one
/// otherwise — which is what makes `╬` four corners around a hole and `╠`'s
/// inner stroke stop for the branch instead of walling it off.
fn stroke(
    m: &mut Mask,
    vertical: bool,
    across: (f32, f32),
    along: (f32, f32),
    gap: Option<(f32, f32)>,
) {
    let (a0, a1) = across;
    let mut put = |lo: f32, hi: f32| {
        if hi > lo {
            if vertical {
                m.rect(a0, lo, a1, hi);
            } else {
                m.rect(lo, a0, hi, a1);
            }
        }
    };
    match gap {
        Some((g0, g1)) if g0 > along.0 && g1 < along.1 => {
            put(along.0, g0);
            put(g1, along.1);
        }
        _ => put(along.0, along.1),
    }
}

/// Draw `c` if it is a double-line character.
pub(super) fn draw(m: &mut Mask, c: char) -> bool {
    let Some(a) = TABLE.iter().find(|(k, _)| *k == c).map(|(_, a)| *a) else {
        return false;
    };
    let t = light_thickness(m.h);
    let (vx_a, vx_b) = pair(m.w, t);
    let (hy_a, hy_b) = pair(m.h, t);
    let (w, h) = (m.w as f32, m.h as f32);
    // Where a stroke stops when it meets the perpendicular band: the outer
    // stroke runs past the far side of that band (so the corner closes), the
    // inner stroke stops at its near side (so the corner has a corner).
    let (out_lo, out_hi) = (hy_a as f32, (hy_b + t) as f32);
    let (in_lo, in_hi) = (hy_b as f32, (hy_a + t) as f32);
    let (vout_lo, vout_hi) = (vx_a as f32, (vx_b + t) as f32);
    let (vin_lo, vin_hi) = (vx_b as f32, (vx_a + t) as f32);
    let [up, right, down, left] = a;
    let (cross_v, cross_h) = (up && down, left && right);
    // Exactly one arm on the other axis: a T-junction, where the stroke on
    // the branch's side steps aside for it rather than walling it off.
    let (single_v, single_h) = (up != down, left != right);

    // The OUTER stroke of an arm is the one on the far side from the arm it
    // turns toward: `╔` turns right, so its left vertical is the outer one.
    for (x, outer) in [(vx_a, right), (vx_b, left)] {
        let across = (x as f32, (x + t) as f32);
        let along = match (up, down) {
            (false, false) => continue,
            (true, true) => (0.0, h),
            (true, false) => (
                0.0,
                if cross_h {
                    out_lo
                } else if outer {
                    out_hi
                } else {
                    in_hi
                },
            ),
            (false, true) => (
                if cross_h {
                    out_hi
                } else if outer {
                    out_lo
                } else {
                    in_lo
                },
                h,
            ),
        };
        let gap = if cross_h && cross_v {
            Some((out_lo, out_hi))
        } else if cross_v && single_h && !outer {
            Some((in_hi, in_lo))
        } else {
            None
        };
        stroke(m, true, across, along, gap);
    }
    for (y, outer) in [(hy_a, down), (hy_b, up)] {
        let across = (y as f32, (y + t) as f32);
        let along = match (left, right) {
            (false, false) => continue,
            (true, true) => (0.0, w),
            (true, false) => (
                0.0,
                if cross_v {
                    vout_lo
                } else if outer {
                    vout_hi
                } else {
                    vin_hi
                },
            ),
            (false, true) => (
                if cross_v {
                    vout_hi
                } else if outer {
                    vout_lo
                } else {
                    vin_lo
                },
                w,
            ),
        };
        let gap = if cross_v && cross_h {
            Some((vout_lo, vout_hi))
        } else if cross_h && single_v && !outer {
            Some((vin_hi, vin_lo))
        } else {
            None
        };
        stroke(m, false, across, along, gap);
    }
    true
}

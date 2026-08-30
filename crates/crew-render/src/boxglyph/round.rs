//! The rounded corners `╭ ╮ ╰ ╯` — crew's own frame, four times per card.
//!
//! A corner is one quarter-circle of the same stroke the straight rules use,
//! plus whatever straight tail is left between the arc's ends and the cell
//! edges. The arc is genuinely curved, so it is genuinely antialiased — that
//! is not the softness this module is against. What it fixes is the rest:
//! the tails are whole-pixel rectangles that line up with the `─` and `│`
//! next to them, the stroke is exactly as thick as its neighbours instead of
//! whatever the dilation made of it, and the arc's ends meet the tails.
use super::{centre, light_thickness, Mask};

/// Samples per axis inside one pixel when integrating the arc's coverage.
/// The arc is the only curved thing here and the mask is cached for the life
/// of the atlas entry, so this can afford to be generous.
const SUB: u32 = 8;

/// `(char, [right_of_centre, below_centre])` — which way the arc's centre
/// lies from the corner, i.e. which two arms the character has. `╭` turns
/// from the right arm down, so its centre is to the right and below.
const CORNERS: &[(char, [bool; 2])] = &[
    ('\u{256D}', [true, true]),
    ('\u{256E}', [false, true]),
    ('\u{256F}', [false, false]),
    ('\u{2570}', [true, false]),
];

/// Draw `c` if it is a rounded corner.
pub(super) fn draw(m: &mut Mask, c: char) -> bool {
    let Some(dir) = CORNERS.iter().find(|(k, _)| *k == c).map(|(_, d)| *d) else {
        return false;
    };
    let t = light_thickness(m.h);
    let (vx0, vx1) = centre(m.w, t);
    let (hy0, hy1) = centre(m.h, t);
    let (xc, yc) = (vx0 as f32 + t as f32 / 2.0, hy0 as f32 + t as f32 / 2.0);
    let (w, h) = (m.w as f32, m.h as f32);
    // The arc is as large as the cell allows in both directions, so the
    // corner reads round rather than clipped, and the tails take the rest.
    //
    // Minus one pixel on each axis, deliberately: an arc that ran all the way
    // to the cell edge would hand the neighbouring `─` an ANTIALIASED column
    // to meet — the corner came out at 247 of 255 against a 255 rule, a
    // visible dip at all four corners of every card. Leaving one whole pixel
    // of straight tail on each arm means a corner joins its neighbours at
    // full ink, which is the join this module exists to guarantee.
    let rx = if dir[0] { w - xc } else { xc };
    let ry = if dir[1] { h - yc } else { yc };
    let r = (rx - 1.0).min(ry - 1.0).max(1.0);
    let cx = if dir[0] { xc + r } else { xc - r };
    let cy = if dir[1] { yc + r } else { yc - r };

    // Straight tails: the vertical one runs from the arc's end to the cell
    // edge it points at, and the horizontal one likewise.
    if dir[1] {
        m.rect(vx0 as f32, cy, vx1 as f32, h);
    } else {
        m.rect(vx0 as f32, 0.0, vx1 as f32, cy);
    }
    if dir[0] {
        m.rect(cx, hy0 as f32, w, hy1 as f32);
    } else {
        m.rect(0.0, hy0 as f32, cx, hy1 as f32);
    }
    arc(m, (cx, cy), r, t as f32, dir);
    true
}

/// Integrate the annulus `[r − t/2, r + t/2]` around `(cx, cy)`, restricted
/// to the quadrant the corner turns through, into the mask.
fn arc(m: &mut Mask, (cx, cy): (f32, f32), r: f32, t: f32, dir: [bool; 2]) {
    let (lo, hi) = (r - t / 2.0, r + t / 2.0);
    let step = 1.0 / SUB as f32;
    for py in 0..m.h {
        for px in 0..m.w {
            let mut hits = 0u32;
            for sy in 0..SUB {
                let y = py as f32 + (sy as f32 + 0.5) * step;
                for sx in 0..SUB {
                    let x = px as f32 + (sx as f32 + 0.5) * step;
                    let (dx, dy) = (x - cx, y - cy);
                    // The far side of the centre is where the arms are, and
                    // they are drawn as tails; the arc owns only its quadrant.
                    if (dx <= 0.0) != dir[0] || (dy <= 0.0) != dir[1] {
                        continue;
                    }
                    let d = (dx * dx + dy * dy).sqrt();
                    if d >= lo && d <= hi {
                        hits += 1;
                    }
                }
            }
            if hits > 0 {
                let cov = (hits * 255 / (SUB * SUB)) as u8;
                let i = (py * m.w + px) as usize;
                m.data[i] = m.data[i].max(cov);
            }
        }
    }
}

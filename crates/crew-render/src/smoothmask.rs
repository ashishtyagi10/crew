//! The stem-darkening kernel: how one alpha mask gains its CoreText weight.
//!
//! Apple's "font smoothing" widens a glyph's strokes by a fraction of a
//! pixel. On a coverage bitmap that is a *fractional dilation*: coverage
//! spills out of every pixel into its neighbours in proportion to the
//! strength, so a stroke's antialiased edge reads fuller without the
//! outline itself moving.
//!
//! The spill is anisotropic — full strength horizontally, half vertically —
//! because smoothing chiefly widens vertical stems, and a terminal grid
//! wants glyphs heavier, not taller.
//!
//! The spill *accumulates* into the room a pixel has left rather than
//! replacing its coverage: `out = own + spill·(1 − own)`. A saturating
//! `max(own, spill)` cannot darken a pixel whose own coverage already beats
//! what its neighbour lends it — which is every pixel on a curve's or a
//! diagonal's flank — so `o` and `/` kept their thin rasterized weight while
//! `l` and `H` took the full widening. Accumulating is monotone (it never
//! dims a pixel) and never exceeds full coverage.
//!
//! # Filling the stroke, not spreading it
//!
//! Reported (2026-09-16): "even with heavy smoothing font should be darker,
//! or brighter on dark." It was not, and the measurement said why. The spill
//! used to land on EMPTY pixels too, so every level of the ladder painted a
//! faint halo one pixel out from every stroke: the same eight glyphs went
//! from 373 inked pixels at `off` to 620 at `light`, and while the total ink
//! rose, the ink *per pixel* fell from 0.54 to 0.36 — and `heavy` only
//! climbed back to 0.43, still short of what `off` already had. More ink,
//! spread thinner, is not darker. It is greyer, and it is what the report
//! was describing.
//!
//! So the spill no longer reaches a pixel the outline never touched, and a
//! covered pixel now also deepens in proportion to the strength. Density
//! climbs with the knob instead of falling off it (0.54 → 0.59 → 0.63 →
//! 0.70), the footprint stays exactly the outline's, and the invariant that
//! held only at `off` — nothing inks a pixel the outline did not reach —
//! now holds the whole way up.
//!
//! This is the same finding [`crate::smoothing::DEFAULT_SMOOTH`] made when it
//! turned the darkening off by default: those extra pixels were "a soft edge
//! with nothing bought for it". Turning it off avoided them; this stops
//! making them, which gives the ladder something to be for again.
//!
//! Both polarities come out right from one change, and they have to: coverage
//! is how much INK a pixel holds, and ink is dark on a bright page and light
//! on a dark one. Denser ink is darker one way up and brighter the other.
use glyphon::cosmic_text::{Placement, SwashImage};

/// Vertical spill as a 0–255 fraction of the horizontal spill. Apple's
/// smoothing widens stems, not crossbars, so the vertical axis gets half.
const VERT_RATIO: u32 = 128;

/// Strength → spill calibration, as a 0–255 fraction. Accumulating lays
/// down about 1.4× the ink a saturating `max()` did at the same strength,
/// and `font_smooth` is a persisted, documented knob whose 100 means
/// "Terminal.app's default look". Scaling the spill by 0.70 keeps that
/// promise: the same number still renders the same weight, and what
/// changed is where the ink lands, not how much of it there is.
const SPILL_SCALE: u32 = 179;

/// `a · b / 255` on 0–255 coverage, rounded rather than truncated — the
/// kernel chains two of these, and truncating both biased every dilated
/// pixel down by up to 2/255 of coverage.
fn mul255(a: u32, b: u32) -> u32 {
    (a * b + 127) / 255
}

/// Apple-style stem darkening on an 8-bit alpha mask. The bitmap grows by a
/// 1 px border (the dilation bleeds past the tight crop) and the placement
/// shifts to compensate, so glyphs do not move.
pub(crate) fn smooth_mask(image: &SwashImage, strength: u8) -> SwashImage {
    let w = image.placement.width as usize;
    let h = image.placement.height as usize;
    if w == 0 || h == 0 {
        return image.clone();
    }
    let (nw, nh) = (w + 2, h + 2);
    let src = |x: isize, y: isize| -> u32 {
        if x < 0 || y < 0 || x >= w as isize || y >= h as isize {
            0
        } else {
            u32::from(image.data[y as usize * w + x as usize])
        }
    };
    let s = mul255(u32::from(strength), SPILL_SCALE);
    let sv = mul255(s, VERT_RATIO);
    let mut data = vec![0u8; nw * nh];
    for y in 0..nh {
        for x in 0..nw {
            let (sx, sy) = (x as isize - 1, y as isize - 1);
            let horiz = mul255(src(sx - 1, sy).max(src(sx + 1, sy)), s);
            let vert = mul255(src(sx, sy - 1).max(src(sx, sy + 1)), sv);
            let own = src(sx, sy);
            // A pixel the outline never reached takes nothing: the stroke is
            // filled, not spread (see the module docs).
            let spill = match own {
                0 => 0,
                _ => horiz.max(vert),
            };
            let out = (own + mul255(spill, 255 - own)).min(255);
            // And what the ink already covers deepens with the strength, so
            // the knob has somewhere to go once the rim is full. Full
            // coverage is already everything a pixel has to give.
            data[y * nw + x] = (out + mul255(mul255(out, s), 255 - out)).min(255) as u8;
        }
    }
    SwashImage {
        source: image.source,
        content: image.content,
        placement: Placement {
            left: image.placement.left - 1,
            top: image.placement.top + 1,
            width: nw as u32,
            height: nh as u32,
        },
        data,
    }
}

#[cfg(test)]
#[path = "smoothmask_tests.rs"]
mod tests;

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
//! dims a pixel), never exceeds full coverage, and is identical to the old
//! kernel wherever a pixel starts empty.
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
            let spill = horiz.max(vert);
            data[y * nw + x] = (own + mul255(spill, 255 - own)).min(255) as u8;
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

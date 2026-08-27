//! How much of the nominal smoothing strength a glyph of a given size takes.
//!
//! Split from [`crate::smoothmask`] because it answers a different question:
//! the kernel says how the ink spreads, this says how much of it there is to
//! spread at this size.
/// The pixel size the strength ladder was calibrated at — crew's default
/// body size. At and above it the user's number is used untouched.
const CALIBRATED_PX: f32 = 14.0;

/// Strength shed per pixel of size below the reference.
const SHED_PER_PX: f32 = 0.04;

/// Floor on the shedding, so text small enough to need every help it can
/// get still gets some.
const MIN_SIZE_SCALE: f32 = 0.6;

/// How much of the nominal strength a glyph at `px` physical pixels takes.
///
/// The spill is a fixed fraction of a pixel, but a stroke is not: it thins
/// with the size, so the same 0.27 px is a larger and larger share of it as
/// the text gets smaller. Measured on the embedded font, a run of body
/// letters gains 31% ink at 14 px and 39% at 9 px from the same strength —
/// and that surplus comes out of the counters, the enclosed white in `e`,
/// `a`, `o`, `8`, which at 9 px are a pixel or two across to begin with.
/// Below the reference they were losing a third of their open area to the
/// darkening, against a seventh at 32 px.
///
/// So the ramp holds the calibration flat down the small end rather than
/// letting it run ahead. It is deliberately one-sided: above the reference
/// the share falls on its own (10% ink at 48 px), and that is correct —
/// large text is rasterized accurately and never needed the help.
///
/// The size is PHYSICAL pixels, which is the domain the dilation lives in.
/// A Retina page at 14 pt rasterizes at 28 px and is already past the
/// reference, which is why its glyphs read fine without this.
pub(crate) fn size_scale(px: f32) -> f32 {
    if px >= CALIBRATED_PX {
        1.0
    } else {
        (1.0 - (CALIBRATED_PX - px) * SHED_PER_PX).max(MIN_SIZE_SCALE)
    }
}

/// The strength a glyph of size `px` is actually dilated by.
pub(crate) fn strength_at(strength: u8, px: f32) -> u8 {
    (f32::from(strength) * size_scale(px))
        .round()
        .clamp(0.0, 255.0) as u8
}

#[cfg(test)]
#[path = "sizeramp_tests.rs"]
mod tests;

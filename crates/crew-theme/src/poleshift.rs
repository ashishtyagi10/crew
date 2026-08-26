//! The live hue offset the gradient poles wear this frame — the one number
//! that makes crew's gradient a *colour that moves* rather than a fixed pair
//! of swatches.
//!
//! Every gradient surface in crew (the page's wash, the dot lattice, every
//! card's stroke, the footer meters) is drawn between the active theme's two
//! `ModernStyle` poles. Those are constants, so until now the whole canvas
//! could only change colour by changing theme. This module rotates both poles
//! around the hue wheel by a shared offset the app advances over time, so the
//! page's light warms and cools on its own.
//!
//! ## What is allowed to move, and what is not
//!
//! The rotation happens in OKLCH, and it moves **H only**: `l` and `c` come
//! back out of [`crate::oklch::from_srgb`] and go straight back in. That is
//! not a stylistic choice, it is the whole safety argument —
//!
//! * every contrast guarantee in the palette suite is a function of
//!   LIGHTNESS, and OKLCH lightness is perceptual, so a pure hue rotation
//!   cannot move a measured ratio the way an RGB rotation would;
//! * `to_srgb` reduces chroma rather than clipping channels when a hue is
//!   less saturated in sRGB than the one it came from, so the worst a
//!   rotation can do is make a pole slightly *less* colourful — never
//!   brighter, never darker, never a different hue than the one asked for.
//!
//! Both poles turn by the SAME offset, in the same direction: the interval
//! between them is the theme's signature (orchid→rose, teal→sky), and a pair
//! that counter-rotated would collapse to one colour twice a cycle and invert
//! in between. The pair leans; it does not scramble.
//!
//! ## Determinism
//!
//! At shift `0` — which is what a fresh process, `gradient off`, Motion off
//! and every headless shot test see — [`shifted`] returns its input
//! bit-for-bit and [`poles`] returns the theme's own constants. The
//! static-frame contract the CRT trace and the modern ring keep is therefore
//! untouched by this module existing.
use std::sync::atomic::{AtomicU32, Ordering};

use crate::oklch;

/// The two ends of a gradient, in sRGB — `(pole_a, pole_b)`. A name for the
/// pair so callers read as "the poles" rather than as a nested tuple.
pub type Poles = ((u8, u8, u8), (u8, u8, u8));

/// The widest offset that can be stored, in degrees either way. A quarter of
/// the wheel: past that a violet theme visits green, and the palette stops
/// being recognisably itself.
pub const MAX_SHIFT_DEG: f32 = 90.0;

/// Live hue offset in degrees, as `f32` bits. Written once per drawn frame by
/// the app and read by every gradient surface, the same shape the theme
/// selection itself uses.
static SHIFT: AtomicU32 = AtomicU32::new(0);

/// Adopt `deg` as the offset every gradient pole now wears. Clamped to
/// [`MAX_SHIFT_DEG`]; a non-finite value is taken as zero, so a bad number
/// upstream freezes the colour rather than corrupting it.
pub fn set_shift(deg: f32) {
    let d = if deg.is_finite() {
        deg.clamp(-MAX_SHIFT_DEG, MAX_SHIFT_DEG)
    } else {
        0.0
    };
    SHIFT.store(d.to_bits(), Ordering::Relaxed);
}

/// The live hue offset in degrees.
pub fn shift() -> f32 {
    f32::from_bits(SHIFT.load(Ordering::Relaxed))
}

/// `rgb` rotated `deg` degrees around the hue wheel at unchanged OKLCH
/// lightness. Exactly `rgb` at `deg == 0`, and on a neutral grey at any
/// offset — a colour with no chroma has no hue to turn.
pub fn shifted(rgb: (u8, u8, u8), deg: f32) -> (u8, u8, u8) {
    if deg == 0.0 || !deg.is_finite() {
        return rgb;
    }
    let c = oklch::from_srgb(rgb);
    if c.c <= 0.001 {
        return rgb;
    }
    oklch::Oklch::new(c.l, c.c, (c.h + deg).rem_euclid(360.0)).to_srgb()
}

/// The active theme's two gradient poles, wearing the live shift. `None` on a
/// theme without a `ModernStyle` — no theme ships without one today, but the
/// field is an `Option` and callers already have a flat-colour fallback.
pub fn poles() -> Option<Poles> {
    let m = crate::theme().modern?;
    let d = shift();
    Some((shifted(m.pole_a, d), shifted(m.pole_b, d)))
}

#[cfg(test)]
#[path = "poleshift_tests.rs"]
mod tests;

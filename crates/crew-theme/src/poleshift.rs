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
//! ## A gradient of the user's own
//!
//! [`set_custom`] replaces the theme's pair with one the user picked. Only
//! their HUE and CHROMA are taken — the LIGHTNESS stays the theme's, at read
//! time, because the theme underneath rotates and because the wash lies under
//! the text with almost no contrast headroom to spend. The user chooses the
//! colour; crew chooses how bright it is.
//!
//! ## Determinism
//!
//! At shift `0` — which is what a fresh process, `gradient off`, Motion off
//! and every headless shot test see — [`shifted`] returns its input
//! bit-for-bit and [`poles`] returns the theme's own constants. The
//! static-frame contract the CRT trace and the modern ring keep is therefore
//! untouched by this module existing.
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::oklch;

/// The two ends of a gradient, in sRGB — `(pole_a, pole_b)`. A name for the
/// pair so callers read as "the poles" rather than as a nested tuple.
pub type Poles = ((u8, u8, u8), (u8, u8, u8));

/// The widest offset that can be stored, in degrees either way. A quarter of
/// the wheel: past that a violet theme visits green, and the palette stops
/// being recognisably itself.
pub const MAX_SHIFT_DEG: f32 = 90.0;

/// The user's own gradient, when they have set one: both poles packed into
/// one `u64` (`1 << 48` marks it present) so a read is a single atomic load —
/// this is read once per card per frame, and a lock there would be a lock in
/// the render path.
static CUSTOM: AtomicU64 = AtomicU64::new(0);

const CUSTOM_SET: u64 = 1 << 48;

fn pack((a, b): Poles) -> u64 {
    let one = |(r, g, b): (u8, u8, u8)| (u64::from(r) << 16) | (u64::from(g) << 8) | u64::from(b);
    CUSTOM_SET | (one(a) << 24) | one(b)
}

fn unpack(v: u64) -> Option<Poles> {
    (v & CUSTOM_SET != 0).then(|| {
        let one = |x: u64| ((x >> 16) as u8, (x >> 8) as u8, x as u8);
        (one((v >> 24) & 0xff_ffff), one(v & 0xff_ffff))
    })
}

/// Adopt a pair of poles of the user's own, or `None` to go back to the
/// theme's. Stored RAW: the re-lighting in [`poles`] has to happen at READ
/// time, because the theme underneath rotates every ten minutes and a colour
/// lit for the page it was set on would be wrong for the next one.
pub fn set_custom(poles: Option<Poles>) {
    CUSTOM.store(poles.map_or(0, pack), Ordering::Relaxed);
}

/// The user's own poles, as they typed them — before the re-lighting.
pub fn custom() -> Option<Poles> {
    unpack(CUSTOM.load(Ordering::Relaxed))
}

/// `user`'s hue and chroma at `reference`'s OKLCH lightness.
///
/// This is the whole safety story for a user-chosen gradient. The wash lies
/// under the text, and it has only 4-16% contrast headroom over the page it
/// lifts (v0.18.26) — a pole a few steps brighter than the theme's own would
/// spend headroom that is not there, and `#ffffff` would erase the page. So
/// the user chooses the COLOUR and crew chooses how bright it is: every
/// measured ratio in the palette suite is a function of lightness, and this
/// holds lightness exactly where the theme put it.
///
/// Chroma is the user's, capped only by what sRGB can show at that lightness
/// (`to_srgb` reduces it rather than clipping a channel). A grey stays grey.
fn relight(user: (u8, u8, u8), reference: (u8, u8, u8)) -> (u8, u8, u8) {
    let u = oklch::from_srgb(user);
    let r = oklch::from_srgb(reference);
    oklch::Oklch::new(r.l, u.c, u.h).to_srgb()
}

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
    let (a, b) = match custom() {
        // The user's colour, at the theme's lightness — see `relight`.
        Some((ca, cb)) => (relight(ca, m.pole_a), relight(cb, m.pole_b)),
        None => (m.pole_a, m.pole_b),
    };
    let d = shift();
    Some((shifted(a, d), shifted(b, d)))
}

#[cfg(test)]
#[path = "poleshift_tests.rs"]
mod tests;

//! Derives the search-highlight wash from the page, so a match is equally
//! findable in every theme.
//!
//! ## What was wrong
//!
//! `find_hl_bg` is the one background colour a palette ships, and it was the
//! one role the 2026-08-22 ramp work never touched — the ramp derives the
//! *text* ladder and this is a wash sitting behind text. So it stayed hand-
//! picked, and measured across the nine palettes it splits cleanly by
//! appearance:
//!
//! | page | Δ page → highlight |
//! |---|---|
//! | blossom | 0.106 |
//! | paper-light | 0.110 |
//! | sepia-light | 0.134 |
//! | nebula | 0.173 |
//! | crt-blue | 0.190 |
//! | sepia-dark | 0.195 |
//! | crt-amber | 0.226 |
//! | paper-dark | 0.230 |
//! | crt-green | 0.246 |
//!
//! On [the distance scale](crate::oklch::distance) 0.10 is *one rung of the
//! text hierarchy* — ink to text_muted. So the light pages' highlight was a
//! single rung off the page while the tubes' was two and a half, and
//! `paper-light` bottomed out at **1.25:1**, which is a search highlight you
//! have to hunt for. It reads that way for a reason: on a dark page a wash
//! can go up in lightness AND gain chroma, while on a bright one the presets
//! reached for a pale yellow that barely moves off the paper.
//!
//! ## A floor, not a target
//!
//! The ramp re-derived every role to a house median because it was building a
//! system out of guesses. This is narrower: five palettes already agree, and
//! the fix is that the other four should be no harder to spot. So [`FLOOR`] is
//! the measured **median** applied as a minimum — a palette already at or past
//! it is returned untouched, and only the four below it move. crew does not
//! end up looking different; it ends up with no theme where find is a
//! guessing game.
//!
//! ## The hue is the palette's, the distance is the house's
//!
//! Same division the ramp settled on. A preset declares what colour its
//! highlight *is* — sepia's amber, nebula's violet, crt-green's phosphor — and
//! this supplies only how far off the page it sits, by scaling the page →
//! highlight vector in OKLab (where distance is Euclidean, so scaling the
//! vector scales the distance exactly). Nothing here invents a hue, which is
//! the mistake the ramp's own docs record.
use crate::oklch::{self, Oklch};

/// Minimum perceptual distance from the page to the highlight wash: the
/// measured median of the nine shipped palettes. See the module docs for why
/// this is a floor rather than a target.
pub const FLOOR: f32 = 0.19;

/// The wash `page` should carry, given the palette's declared highlight.
///
/// Returns `declared` unchanged when it already clears [`FLOOR`]. Otherwise
/// pushes it along its own page → highlight direction until it does. A
/// declared colour equal to the page has no direction to push along and is
/// returned as-is — a palette that ships no highlight is a palette bug, and
/// inventing a hue for it here would hide that from the parity test.
pub fn wash(page: (u8, u8, u8), declared: (u8, u8, u8)) -> (u8, u8, u8) {
    let have = oklch::distance(page, declared);
    if have >= FLOOR {
        return declared;
    }
    if have < 1e-6 {
        // No direction to scale along. See the doc comment.
        return declared;
    }
    let (p, d) = (oklch::from_srgb(page), oklch::from_srgb(declared));
    let (pl, pa, pb) = lab(p);
    let (dl, da, db) = lab(d);
    let k = FLOOR / have;
    from_lab(pl + (dl - pl) * k, pa + (da - pa) * k, pb + (db - pb) * k)
}

/// OKLCH → OKLab components (the space `distance` measures in).
fn lab(c: Oklch) -> (f32, f32, f32) {
    let rad = c.h.to_radians();
    (c.l, c.c * rad.cos(), c.c * rad.sin())
}

/// OKLab components → sRGB, through `Oklch::to_srgb`'s gamut mapping.
fn from_lab(l: f32, a: f32, b: f32) -> (u8, u8, u8) {
    Oklch::new(
        l,
        (a * a + b * b).sqrt(),
        b.atan2(a).to_degrees().rem_euclid(360.0),
    )
    .to_srgb()
}

#[cfg(test)]
#[path = "highlight_tests.rs"]
mod tests;

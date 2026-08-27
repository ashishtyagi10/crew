//! The other half of CoreText's font smoothing: the coverage curve.
//!
//! Crew deliberately picks a NON-sRGB surface, so text blends on
//! gamma-encoded values (see [`crate::gpu::pick_surface_format`]) — the web
//! and CoreText look. That choice has a cost nothing was paying for: a
//! partial-coverage pixel at alpha `a` lands at `a` in *encoded* space, which
//! is `a^2.2` of the light it should emit. White text on a dark page
//! therefore delivers only about 60% of its correct linear luminance
//! (measured over the embedded font at body size), and reads thin. Dark text
//! on a bright page has the same error with the sign flipped: its edges come
//! out darker than they should, and the letters read blotted.
//!
//! Apple corrects this with a contrast curve applied to the mask, bent by the
//! polarity of the text. So does crew, on the same 0–255 knob idiom as
//! `/smooth`:
//!
//! - on a dark page  `a' = a^(1/2.2)`  — lift the edges back up,
//! - on a bright page `a' = 1 − (1 − a)^(1/2.2)` — let them back down,
//!
//! blended `amount/255` of the way from the identity, so the knob runs from
//! "leave the blend alone" to the full physical correction. Both curves fix
//! 0 and 1: a glyph's empty pixels and its solid interior never move, and
//! only the antialiased rim — which is most of a small glyph — is touched.

/// Display gamma the encoded blend is losing to.
const GAMMA: f32 = 2.2;

/// Default correction amount (0–255). About half the full sRGB correction,
/// which puts the midtone at Apple's historical 1/1.45 text gamma — enough to
/// return the ink the blend eats without tipping body text into looking bold.
pub const DEFAULT_TEXT_GAMMA: u8 = 130;

/// The 256-entry coverage curve for one polarity and amount. Cheap enough
/// (256 `powf`) to build once per frame per distinct pair; [`Curve`] does the
/// remembering.
fn build(dark: bool, amount: u8) -> [u8; 256] {
    let f = f32::from(amount) / 255.0;
    let mut lut = [0u8; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        let a = i as f32 / 255.0;
        let full = if dark {
            a.powf(1.0 / GAMMA)
        } else {
            1.0 - (1.0 - a).powf(1.0 / GAMMA)
        };
        *slot = (255.0 * (a + f * (full - a))).round().clamp(0.0, 255.0) as u8;
    }
    lut
}

/// A coverage curve that remembers the pair it was built for. `presmooth`
/// walks a frame's glyphs in cache-key order, and every glyph in a frame
/// shares one polarity and amount, so this rebuilds once and then hits.
pub(crate) struct Curve {
    key: Option<(bool, u8)>,
    lut: [u8; 256],
}

impl Curve {
    pub(crate) fn new() -> Self {
        Self {
            key: None,
            lut: [0; 256],
        }
    }

    /// Apply the curve for `(dark, amount)` to an alpha mask in place. Amount
    /// 0 is the identity and is skipped outright.
    pub(crate) fn apply(&mut self, data: &mut [u8], dark: bool, amount: u8) {
        if amount == 0 {
            return;
        }
        if self.key != Some((dark, amount)) {
            self.lut = build(dark, amount);
            self.key = Some((dark, amount));
        }
        for a in data.iter_mut() {
            *a = self.lut[*a as usize];
        }
    }
}

#[cfg(test)]
#[path = "textgamma_tests.rs"]
mod tests;

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

/// Whether a cell paints light ink on a darker ground — which way its
/// coverage curve has to bend. Read per RUN rather than per theme: crew
/// draws dark text on bright badges inside dark themes and vice versa, and
/// the cell under an inverted cursor flips both at once. A theme-wide
/// polarity gets every one of those backwards, which is the direction that
/// makes text look worst, since the correction then doubles the error it
/// was there to cancel.
pub(crate) fn light_ink(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> bool {
    crew_theme::relative_luminance(fg) > crew_theme::relative_luminance(bg)
}

/// Display gamma the encoded blend is losing to.
const GAMMA: f32 = 2.2;

/// Default correction amount (0–255) — the **full** sRGB correction.
///
/// This was 130, about half, and half was the right answer while the stem
/// darkening in [`crate::smoothmask`] was still on by default and delivering
/// the other half. With the darkening off (see [`crate::smoothing::DEFAULT_SMOOTH`])
/// the blend's error is this curve's alone to cancel, and cancelling it
/// completely is what puts a glyph's delivered light exactly on the light
/// its outline asked for — 100.0% on a dark page and 100.0% on a bright one,
/// measured over eight glyphs at two sizes.
///
/// Full does not mean heavy: the curve fixes 0 and 1, so it moves only the
/// antialiased rim, and it moves it in whichever direction that rim is
/// wrong. On a bright page it takes ink AWAY (the old pair delivered 145% of
/// what the outline asked for there), which is the correction reading as
/// *crisper*, not bolder.
pub const DEFAULT_TEXT_GAMMA: u8 = 255;

/// The share of the full correction a run's OWN colours ask for, as a
/// 0.0–1.0 factor on the amount.
///
/// The curve above is exact for the extreme pair — white ink on a black page
/// or the reverse — because that is the pair `a^(1/2.2)` describes. Crew's
/// pages are not that pair. Its dark page is (24, 20, 17) under ink around
/// (230, 226, 218), and a muted comment, a dim border legend or a badge sits
/// closer still. Over a narrower span the encoded blend loses proportionally
/// less light, so the full correction OVERSHOOTS, and it overshoots most at
/// the low end: a coverage of 0.05 is lifted to 0.26 when the pair only
/// asks for 0.19. That lift lands on the outermost pixel of every stroke,
/// which is exactly where a halo comes from.
///
/// The exact per-pair correction is not a power curve at all — it is
///
/// ```text
/// a' = ((a·Lf + (1−a)·Lb)^(1/γ) − Gb) / (Gf − Gb)
/// ```
///
/// — but across a whole glyph it tracks the power curve scaled toward the
/// identity, within a couple of percent. Matching the two at half coverage
/// gives the factor, and scaling the AMOUNT by it needs no new plumbing at
/// all: the amount is already a byte in every glyph's cache key, so a run
/// whose colours ask for less correction simply mints its own keys.
///
/// Colours are compared by relative luminance. A mask is one channel and the
/// error is per-channel, so no single number can be right for all three;
/// luminance is the one the eye weights the answer by.
pub(crate) fn contrast_factor(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> f32 {
    let (lf, lb) = (
        crew_theme::relative_luminance(fg),
        crew_theme::relative_luminance(bg),
    );
    let (gf, gb) = (lf.powf(1.0 / GAMMA), lb.powf(1.0 / GAMMA));
    let span = gf - gb;
    // Ink the same luminance as its ground has no visible rim to correct.
    if span.abs() < 1e-3 {
        return 0.0;
    }
    let half = (0.5 * lf + 0.5 * lb).powf(1.0 / GAMMA);
    let exact = (half - gb) / span;
    let full = if lf > lb {
        0.5_f32.powf(1.0 / GAMMA)
    } else {
        1.0 - 0.5_f32.powf(1.0 / GAMMA)
    };
    ((exact - 0.5) / (full - 0.5)).clamp(0.0, 1.0)
}

/// The amount one run takes of the configured correction — [`contrast_factor`]
/// applied to it and rounded back to the byte the cache key carries.
pub(crate) fn amount_for(base: u8, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> u8 {
    (f32::from(base) * contrast_factor(fg, bg)).round() as u8
}

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

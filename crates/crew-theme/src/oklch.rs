//! OKLCH ⇄ sRGB, and the one thing the palettes actually need from it: *solve
//! for the colour that hits a contrast ratio*.
//!
//! ## Why a perceptual space at all
//!
//! Every role in every preset is a hand-picked sRGB triple, and sRGB lightness
//! is not perceived lightness. Two colours with the same HSL `L` at different
//! hues can differ by a factor of three in luminance — which is why a palette
//! recipe tuned on a blue accent falls apart the moment the same recipe is
//! applied to amber, and why the 24 presets could only ever be tuned one at a
//! time by eye.
//!
//! OKLab (Björn Ottosson, 2020) fixes that: equal `L` reads as equal
//! lightness across hues. OKLCH is its cylindrical form — **L**ightness 0..1,
//! **C**hroma (0 = grey, and how far it can go depends on hue and L), **H**ue
//! in degrees. Hold C and H, vary L, and you get a tonal ramp whose steps look
//! evenly spaced. That is the whole trick behind the 50–950 scales in
//! Tailwind, Radix and Material, and it is what `ramp` uses to derive a
//! palette's roles instead of guessing them.
//!
//! ## Gamut
//!
//! OKLCH can name colours sRGB cannot show. [`Oklch::to_srgb`] does not clip
//! channels — clipping shifts hue visibly — it **reduces chroma** until the
//! colour fits, keeping L and H, which is the standard CSS Color 4 approach
//! and preserves the two properties the ramp is reasoning about.
use crate::contrast_ratio;

/// A colour as lightness, chroma and hue.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklch {
    /// Perceptual lightness, 0 (black) to 1 (white).
    pub l: f32,
    /// Colourfulness. 0 is neutral grey; sRGB tops out near 0.32 and the
    /// reachable maximum depends on both hue and lightness.
    pub c: f32,
    /// Hue angle in degrees, 0..360.
    pub h: f32,
}

impl Oklch {
    pub const fn new(l: f32, c: f32, h: f32) -> Self {
        Self { l, c, h }
    }

    /// The same colour with a different lightness.
    pub fn with_l(self, l: f32) -> Self {
        Self { l, ..self }
    }

    /// The same colour with a different chroma.
    pub fn with_c(self, c: f32) -> Self {
        Self { c, ..self }
    }

    /// Convert to sRGB, reducing chroma (never clipping channels) until the
    /// colour is representable. Hue and lightness are preserved.
    pub fn to_srgb(self) -> (u8, u8, u8) {
        if let Some(rgb) = self.try_srgb(self.c) {
            return rgb;
        }
        // Binary search the largest in-gamut chroma. 12 steps resolves finer
        // than one 8-bit code, so the result is exact for our purposes.
        let (mut lo, mut hi) = (0.0f32, self.c);
        let mut best = self.try_srgb(0.0).unwrap_or((0, 0, 0));
        for _ in 0..12 {
            let mid = 0.5 * (lo + hi);
            match self.try_srgb(mid) {
                Some(rgb) => {
                    best = rgb;
                    lo = mid;
                }
                None => hi = mid,
            }
        }
        best
    }

    /// sRGB for this colour at chroma `c`, or `None` when out of gamut.
    fn try_srgb(self, c: f32) -> Option<(u8, u8, u8)> {
        let (r, g, b) = self.with_c(c).to_linear_rgb();
        let enc = |v: f32| -> Option<u8> {
            // A hair outside is rounding, not an out-of-gamut colour.
            if !(-1e-4..=1.0 + 1e-4).contains(&v) {
                return None;
            }
            let v = v.clamp(0.0, 1.0);
            let s = if v <= 0.003_130_8 {
                v * 12.92
            } else {
                1.055 * v.powf(1.0 / 2.4) - 0.055
            };
            Some((s * 255.0).round().clamp(0.0, 255.0) as u8)
        };
        Some((enc(r)?, enc(g)?, enc(b)?))
    }

    /// Linear-light RGB, possibly outside 0..1 (i.e. outside the sRGB gamut).
    fn to_linear_rgb(self) -> (f32, f32, f32) {
        let (a, b) = {
            let rad = self.h.to_radians();
            (self.c * rad.cos(), self.c * rad.sin())
        };
        // OKLab → LMS' → LMS → linear sRGB (Ottosson's matrices).
        let l_ = self.l + 0.396_337_78 * a + 0.215_803_76 * b;
        let m_ = self.l - 0.105_561_346 * a - 0.063_854_17 * b;
        let s_ = self.l - 0.089_484_18 * a - 1.291_485_5 * b;
        let (l, m, s) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
        (
            4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s,
            -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s,
            -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s,
        )
    }
}

/// sRGB → OKLCH.
pub fn from_srgb(rgb: (u8, u8, u8)) -> Oklch {
    let lin = |c: u8| -> f32 {
        let x = c as f32 / 255.0;
        if x <= 0.040_45 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (lin(rgb.0), lin(rgb.1), lin(rgb.2));
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_5 * b).cbrt();
    let lightness = 0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s;
    let a = 1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s;
    let bb = 0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s;
    let c = (a * a + bb * bb).sqrt();
    let h = if c < 1e-6 {
        0.0
    } else {
        bb.atan2(a).to_degrees().rem_euclid(360.0)
    };
    Oklch::new(lightness, c, h)
}

/// Perceptual distance between two colours (Euclidean in OKLab).
///
/// Calibrated against crew's own palettes (see the scale test), so the numbers
/// mean something concrete here rather than in the abstract:
///
/// | Δ | what it looks like |
/// |---|---|
/// | 0.0015 | one 8-bit code — invisible |
/// | 0.027 | eight codes of grey — a visible step |
/// | **0.10** | AURORA `ink` → `text_muted` — one rung of the text hierarchy |
/// | 0.15 | `text_muted` → `legend_off` |
/// | 0.20 | AURORA ANSI red vs yellow — never confusable |
/// | 1.00 | black vs white |
///
/// Used to bound how far a derived palette may drift from the hand-tuned one
/// it replaces, and to check ANSI slots stay distinguishable.
pub fn distance(a: (u8, u8, u8), b: (u8, u8, u8)) -> f32 {
    let (x, y) = (from_srgb(a), from_srgb(b));
    let ab = |p: Oklch| {
        let rad = p.h.to_radians();
        (p.c * rad.cos(), p.c * rad.sin())
    };
    let (ax, bx) = ab(x);
    let (ay, by) = ab(y);
    ((x.l - y.l).powi(2) + (ax - ay).powi(2) + (bx - by).powi(2)).sqrt()
}

/// Which way a role sits from its background.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toward {
    /// Lighter than the background — a role on a dark page.
    Light,
    /// Darker than the background — a role on a light page.
    Dark,
}

impl Toward {
    /// The direction ink must travel to be legible on `page`.
    pub fn for_page(page: (u8, u8, u8)) -> Self {
        if from_srgb(page).l < 0.5 {
            Toward::Light
        } else {
            Toward::Dark
        }
    }
}

/// The colour at `hue`/`chroma` whose WCAG contrast against `bg` is at least
/// `target`, as close to `bg` as that allows.
///
/// This is the inversion the whole ramp rests on. `contrast_thresholds` used
/// to be a check applied to numbers someone had already chosen; here the
/// required ratio is the *input* and the colour is the output, so a role
/// cannot be short of its floor by construction.
///
/// "As close to `bg` as allowed" is deliberate: a muted role should sit at its
/// floor, not beyond it, or every theme drifts toward maximum contrast and the
/// hierarchy between `ink`, `text_muted` and `hint_fg` flattens out.
///
/// Returns the extreme (white or black) when even that cannot reach `target` —
/// the caller gets the most legible colour available rather than an error, and
/// the independent suite still catches it if that is not enough.
pub fn solve_for_contrast(
    bg: (u8, u8, u8),
    hue: f32,
    chroma: f32,
    target: f32,
    toward: Toward,
) -> (u8, u8, u8) {
    let at = |l: f32| Oklch::new(l, chroma, hue).to_srgb();
    // Search between the background's own lightness and the extreme, so the
    // result is the closest qualifying colour rather than the farthest.
    let bg_l = from_srgb(bg).l;
    let (mut near, mut far) = match toward {
        Toward::Light => (bg_l, 1.0),
        Toward::Dark => (bg_l, 0.0),
    };
    if contrast_ratio(at(far), bg) < target {
        return at(far); // even the extreme falls short; hand back the best there is
    }
    // `near` fails and `far` passes; converge on the boundary. 16 steps put us
    // well inside one 8-bit code.
    for _ in 0..16 {
        let mid = 0.5 * (near + far);
        if contrast_ratio(at(mid), bg) >= target {
            far = mid;
        } else {
            near = mid;
        }
    }
    at(far)
}

#[cfg(test)]
#[path = "oklch_tests.rs"]
mod tests;

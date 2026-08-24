//! The 16-slot terminal palette, derived rather than eyeballed.
//!
//! ## What was wrong
//!
//! `contrast_thresholds` checks slots 1–6 and 9–14 against `term_bg` at ≥ 3.0
//! and skips the rest, which left three holes that measurement filled in:
//!
//! * **Slot 0 bottoms out at 1.36:1.** Anything a program prints in ANSI black
//!   on a dark theme is very nearly the background.
//! * **The floor has nothing above it.** Slots range from 4.64 to 17.34
//!   against their own background depending on theme and slot — the same
//!   "muted means two different things" problem Phase 1 fixed for the text
//!   ladder, transposed onto shell output.
//! * **Nothing compares the slots to *each other*.** On `crt-amber`, ANSI
//!   green `(240, 200, 40)` and yellow `(255, 200, 30)` sit **Δ 0.0227**
//!   apart — closer than eight codes of grey, which is the point at which two
//!   greys stop being separable. `ls --color` and `git diff` draw with these.
//!
//! ## Two ways to tell sixteen things apart
//!
//! A palette needs its slots *distinguishable*, and there are two honest ways
//! to get there — which is why forcing one scheme on both pools would repeat
//! the Phase 1 mistake of flattening the CRT tubes into something they are not.
//!
//! [`AnsiMode::Chromatic`] spreads the six chromatic slots **by hue**, at one
//! shared contrast so no colour shouts louder than another. The hues are
//! crew's own, measured as the median across the 19 non-CRT presets rather
//! than taken from a standard: red 27.5°, yellow 84.5°, green 149.1°,
//! cyan 199.4°, blue 253.3°, magenta 323.2°.
//!
//! [`AnsiMode::Monochrome`] is for the phosphor tubes, where every slot is the
//! *same* hue by design and spreading them would destroy the theme. A real
//! amber monitor distinguished colours by **brightness**, and so does this:
//! six evenly spaced lightness steps along the phosphor's own hue. That is
//! both historically right and the only separation available.
use crate::oklch::{self, Oklch, Toward};

/// crew's canonical hue for each chromatic slot, in OKLCH degrees — the median
/// across the 19 non-CRT presets, so these are what crew already meant by
/// "red" rather than an imported convention.
///
/// Indexed by ANSI slot minus one: red, green, yellow, blue, magenta, cyan.
pub const HUES: [f32; 6] = [27.5, 149.1, 84.5, 253.3, 323.2, 199.4];

/// How a palette separates its slots.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnsiMode {
    /// Six hues around the wheel at one shared contrast.
    Chromatic,
    /// One hue at six lightness steps — a monochrome phosphor tube.
    Monochrome {
        /// The phosphor's hue in OKLCH degrees.
        hue: f32,
        /// Lightness of the brightest slot — the top of the *bright* row. The
        /// rest of the ladder is laid out below it, so a tube keeps the
        /// brightness it already had at its peak.
        hi: f32,
    },
}

/// Contrast targets for the non-chromatic slots, plus the chromatic level.
#[derive(Clone, Copy, Debug)]
pub struct AnsiHouse {
    /// Shared contrast for slots 1–6 against `term_bg` (chromatic mode only).
    pub chromatic: f32,
    /// Slot 0. The current floor of 1.36 is why this exists.
    pub black: f32,
    /// Slot 8.
    pub bright_black: f32,
    /// Slot 7.
    pub white: f32,
    /// Slot 15.
    pub bright_white: f32,
    /// Chroma asked for on the chromatic slots. Gamut reduction lowers it per
    /// hue as needed — asking is how each hue gets as much as it can carry.
    pub chroma: f32,
    /// Lightness added to a base slot to make its bright twin.
    pub bright_step: f32,
    /// Lightness spanned by the six base slots of a monochrome tube.
    ///
    /// A phosphor separates its slots by brightness alone, so this is the only
    /// budget it has. Five gaps must each clear `SEPARABLE` (0.04), hence
    /// 0.20 — and the tubes shipped with far less: `crt-green`'s six slots
    /// spanned 0.111, putting adjacent ones Δ 0.0237 apart, closer than eight
    /// codes of grey. The ladder therefore extends *downward* from each tube's
    /// existing peak rather than pushing brighter, which would blow past the
    /// gamut and wash the phosphor out.
    pub mono_span: f32,
}

/// The ANSI ladder for **dark** pages, measured across the presets 2026-08-22.
pub const ANSI_DARK: AnsiHouse = AnsiHouse {
    chromatic: 9.6,
    // Raised from the measured 2.70. This slot is the reason Phase 2 exists:
    // it bottomed out at 1.36:1, so anything printed in ANSI black on a dark
    // theme was very nearly the background.
    black: 3.2,
    bright_black: 5.4,
    white: 13.6,
    bright_white: 16.8,
    chroma: 0.123,
    bright_step: 0.063,
    mono_span: 0.22,
};

/// The ANSI ladder for **light** pages.
///
/// Not a variation on the dark one — the neutral slots invert outright. On a
/// dark page ANSI black is a dark grey barely above the background (2.7:1
/// measured) and white is bright (13.6); on a light page black is genuine
/// black against the paper (14.7) and "white" has to become a mid grey (8.5)
/// or it would be invisible. The chromatic slots differ too: 6.4 rather than
/// 9.6, because dark ink on a bright page at the dark pool's contrast reads
/// as heavy.
pub const ANSI_LIGHT: AnsiHouse = AnsiHouse {
    chromatic: 6.4,
    black: 14.6,
    bright_black: 5.7,
    white: 8.5,
    bright_white: 14.3,
    chroma: 0.107,
    bright_step: 0.058,
    mono_span: 0.22,
};

/// Derives one theme's 16 slots.
#[derive(Clone, Copy, Debug)]
pub struct AnsiRamp {
    bg: (u8, u8, u8),
    toward: Toward,
    mode: AnsiMode,
    house: AnsiHouse,
}

impl AnsiRamp {
    pub fn new(bg: (u8, u8, u8), mode: AnsiMode, house: AnsiHouse) -> Self {
        Self {
            bg,
            toward: Toward::for_page(bg),
            mode,
            house,
        }
    }

    /// The ramp a shipped preset implies: a phosphor tube keeps its hue and
    /// the lightness span its own slots already occupy; everything else
    /// separates by hue.
    pub fn fitted(t: &crate::Theme) -> Self {
        // A *white* phosphor is not a coloured one: it can show hues, and
        // `crt-paperwhite` ships an ANSI palette that does. Treating it as
        // monochrome painted its red `(255, 180, 175)` blue, which is the
        // Phase 1 mistake — inferring a tube's character from its pool rather
        // than from the tube — repeated one layer down.
        let coloured_phosphor = t.is_tube() && oklch::from_srgb(t.ink).c >= 0.04;
        let mode = if coloured_phosphor {
            // The peak the tube already reaches, across base and bright.
            let hi = (1..7)
                .chain(9..15)
                .map(|i| oklch::from_srgb(t.ansi[i]).l)
                .fold(f32::MIN, f32::max);
            AnsiMode::Monochrome {
                hue: oklch::from_srgb(t.ink).h,
                hi,
            }
        } else {
            AnsiMode::Chromatic
        };
        let house = if t.dark { ANSI_DARK } else { ANSI_LIGHT };
        Self::new(t.term_bg, mode, house)
    }

    /// How this palette separates its slots. Exposed so a test asks the ramp
    /// rather than re-deriving the rule and drifting from it.
    pub fn mode(&self) -> AnsiMode {
        self.mode
    }

    /// All sixteen slots, in ANSI order.
    pub fn slots(&self) -> [(u8, u8, u8); 16] {
        let mut out = [(0u8, 0u8, 0u8); 16];
        out[0] = self.neutral(self.house.black);
        out[7] = self.neutral(self.house.white);
        out[8] = self.neutral(self.house.bright_black);
        out[15] = self.neutral(self.house.bright_white);
        for i in 0..6 {
            let base = self.chromatic(i);
            out[i + 1] = base;
            out[i + 9] = self.brighten(base);
        }
        out
    }

    /// A grey at `target` contrast, carrying a trace of the background's hue
    /// so the neutral slots belong to the theme.
    fn neutral(&self, target: f32) -> (u8, u8, u8) {
        let bg = oklch::from_srgb(self.bg);
        oklch::solve_for_contrast(self.bg, bg.h, (bg.c * 0.4).min(0.02), target, self.toward)
    }

    /// Chromatic slot `i` (0 = red … 5 = cyan).
    fn chromatic(&self, i: usize) -> (u8, u8, u8) {
        match self.mode {
            AnsiMode::Chromatic => oklch::solve_for_contrast(
                self.bg,
                HUES[i],
                self.house.chroma,
                self.house.chromatic,
                self.toward,
            ),
            AnsiMode::Monochrome { hue, hi } => {
                // The bright row tops out at `hi`, so the base row sits one
                // step below it and spans `mono_span` downward from there.
                let top = hi - self.house.bright_step;
                let lo = top - self.house.mono_span;
                let t = i as f32 / 5.0;
                Oklch::new(lo + (top - lo) * t, self.house.chroma, hue).to_srgb()
            }
        }
    }

    /// The bright twin of a base slot: same hue, one lightness step further
    /// from the background.
    fn brighten(&self, base: (u8, u8, u8)) -> (u8, u8, u8) {
        let c = oklch::from_srgb(base);
        let l = match self.toward {
            Toward::Light => (c.l + self.house.bright_step).min(0.97),
            Toward::Dark => (c.l - self.house.bright_step).max(0.03),
        };
        c.with_l(l).to_srgb()
    }
}

#[cfg(test)]
#[path = "ansi_tests.rs"]
mod tests;

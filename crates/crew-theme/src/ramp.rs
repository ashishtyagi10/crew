//! Derives a palette's text ladder from the page it sits on, so "muted" means
//! the same thing in every theme.
//!
//! ## What was wrong
//!
//! Each preset's roles were picked by eye against that preset's own page. They
//! all clear `contrast_thresholds`, but measured across the 24 themes the same
//! role lands anywhere in a **2× band** of contrast:
//!
//! | role | min | median | max | spread |
//! |---|---|---|---|---|
//! | `ink` | 11.41 | 16.16 | 18.62 | 1.63× |
//! | `text_muted` | 8.21 | 10.95 | 11.62 | 1.42× |
//! | `legend_off` | 4.78 | 5.93 | 7.71 | 1.62× |
//! | `hint_fg` | 3.32 | 4.70 | 6.82 | 2.05× |
//! | `border_normal` | 1.47 | 2.11 | 2.93 | 2.00× |
//!
//! So switching themes does not move the palette to the same place. It moves
//! it to whatever was decided on a different afternoon.
//!
//! ## The targets are medians, not floors
//!
//! The obvious move — solve each role at its `contrast_thresholds` floor — is
//! wrong, and the table above says why: every role already sits far *above*
//! its floor (`ink` at a median 16.2 against a floor of 10). Building to the
//! floors would dim the entire app and fail this work's one rule, which is
//! that crew must not end up looking *different*. So [`HOUSE`] takes the
//! measured medians — what the 24 palettes already agree on when you average
//! out the guessing — and applies them uniformly. The floors stay exactly
//! where they are, as the independent check in `lib_tests.rs` that the ramp
//! did what it claims.
//!
//! ## A theme declares its ink; the ramp only spaces it
//!
//! The first version of this derived the ink's hue from the page, and it was
//! wrong in a way worth recording: it turned `crt-green`'s ink from the
//! phosphor `(0, 255, 102)` into a grey `(225, 229, 228)`. The CRT pool's
//! whole identity is that its text *is* the phosphor, and no amount of
//! inference from a near-black page recovers that.
//!
//! So a palette declares **two** numbers — the hue its text ladder is built
//! on, and how saturated that is ([`Ink`]) — and the ramp supplies only the
//! *spacing*. Seven roles derived from two declared values, instead of seven
//! guessed independently. [`Ink::neutral_for`] covers the ordinary case, where
//! the ladder is the page's own hue at a fraction of its chroma so SEPIA's
//! greys stay warm and AURORA's stay cool.
//! ## What derivation actually changed (2026-08-22)
//!
//! Applying the ramp to all 24 presets moved 20 of them by less than half a
//! rung — imperceptible; the rendered frames differ by under 1% RMSE. Two
//! groups moved further, and both were the palette disagreeing with the
//! system rather than the reverse:
//!
//! * **`daybreak`, `cirrus`, `meadow`, `blossom`** — `dim` across the
//!   non-CRT pool spans 2.67..5.37 with a median of 4.51, and these four sat
//!   at the bottom with an input-bar hint markedly fainter than every other
//!   theme's. Light and dark medians were checked separately before accepting
//!   this: they agree (4.51 light, 4.63 dark), so it is not a light-page
//!   property being flattened. Their frames moved ~1.3% RMSE, the largest
//!   change anywhere.
//! * **`graphite`** — its page sits at L 0.231 against the pool's 0.10..0.19,
//!   so matching the pool's contrast wanted a near-white ink. [`House::max_l`]
//!   holds it to `(241, 241, 243)` at a contrast of 15.0 rather than the
//!   house 16.2. Deliberate: glare is worse than a percent of inconsistency.
//!
//! `crt-paperwhite` was a third case until it got [`HOUSE_CRT_WHITE`]; being
//! bright is that theme's entire point, and holding a white phosphor to the
//! coloured tubes' levels was the bug, not the palette.
use crate::oklch::{self, Toward};

/// Target contrast against the page for each derived role — the median of
/// what the 24 hand-tuned palettes already do. See the module docs for why
/// these are medians rather than the `contrast_thresholds` floors.
#[derive(Clone, Copy, Debug)]
pub struct House {
    /// Which ladder this is, for diagnostics and for grouping in tests. Two
    /// ladders can share a level for one role and differ on another, so the
    /// name is the only reliable identity.
    pub name: &'static str,
    pub ink: f32,
    pub text_muted: f32,
    pub legend_off: f32,
    pub dim: f32,
    pub hint_fg: f32,
    pub placeholder: f32,
    pub border_normal: f32,
    /// How strongly a role's hue drifts toward the page as it descends the
    /// ladder. `1.0` blends fully; `0.0` holds the ink's hue throughout.
    ///
    /// This is a pool property, not a universal one, and assuming otherwise
    /// cost an iteration. Paper and modern palettes want the pull — MEADOW's
    /// border really is greenish like its page rather than blue like its ink,
    /// and something sitting at 2:1 against the page is nearly part of it. A
    /// phosphor wants the opposite: `crt-amber` is amber at *every* rung, and
    /// pulling its `legend_off` toward the near-black page turned
    /// `(180, 115, 20)` into a washed pink `(179, 124, 130)`.
    pub page_pull: f32,
    /// Ceiling on how light a role may get, in OKLCH lightness.
    ///
    /// Contrast alone would push a theme whose page is lighter than its pool's
    /// toward white text: GRAPHITE's page sits at L 0.231 against the pool's
    /// 0.10–0.19, so matching the pool's 16.2 needed an ink of L 0.986 —
    /// `(250, 250, 252)`, effectively white. Near-white text on a dark page is
    /// the glare that every dark-mode guide tells you to avoid, and crew's own
    /// palettes never did it: the brightest shipped ink is `paper-dark`'s L
    /// 0.976 and everything else sits at 0.95 or below.
    ///
    /// 0.96 is `(242, 242, 242)` at neutral. A theme that cannot reach its
    /// target contrast below the cap gets the cap, and lands slightly under
    /// the house ratio — the right trade, since the alternative is glare.
    pub max_l: f32,
}

/// The ladder for crew's paper and modern pools, measured 2026-08-22.
pub const HOUSE: House = House {
    name: "paper/modern",
    ink: 16.2,
    text_muted: 11.2,
    legend_off: 5.9,
    dim: 4.6,
    hint_fg: 4.7,
    placeholder: 4.1,
    border_normal: 2.0,
    page_pull: 1.0,
    max_l: 0.96,
};

/// The ladder for the CRT pool, which genuinely sits lower — a phosphor is a
/// *coloured* ink, and colour costs contrast: pushing `crt-amber`'s
/// `(255, 184, 0)` up to the paper pool's 16.2 drains the chroma out of it and
/// leaves a pale cream. Measured the same way, over the five CRT presets.
pub const HOUSE_CRT: House = House {
    name: "crt (coloured phosphor)",
    ink: 13.4,
    text_muted: 8.5,
    legend_off: 5.9,
    dim: 3.2,
    hint_fg: 5.1,
    placeholder: 4.2,
    border_normal: 2.4,
    page_pull: 0.0,
    max_l: 0.96,
};

/// The ladder for `crt-paperwhite`, which is a **white** phosphor.
///
/// The other four tubes are coloured — green, amber, blue, violet — and a
/// coloured ink costs contrast, which is why `HOUSE_CRT` sits low. Paperwhite
/// pays no such cost and is bright by design; holding it to the coloured
/// tubes' ink level dimmed it from 17.8 to 13.4 and took the "white" out of a
/// theme named for it. So it takes the paper pool's levels, and keeps the CRT
/// pool's `page_pull` of zero because a phosphor holds its hue down the whole
/// ladder.
pub const HOUSE_CRT_WHITE: House = House {
    name: "crt (white phosphor)",
    ink: HOUSE.ink,
    text_muted: HOUSE.text_muted,
    legend_off: HOUSE.legend_off,
    dim: HOUSE.dim,
    hint_fg: HOUSE.hint_fg,
    placeholder: HOUSE.placeholder,
    border_normal: HOUSE_CRT.border_normal,
    page_pull: 0.0,
    max_l: 0.96,
};

/// The hue and saturation a palette's text ladder is built on.
///
/// Declared per theme rather than inferred: see the module docs on what
/// inferring it did to the CRT pool.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ink {
    /// Hue angle in degrees.
    pub hue: f32,
    /// Chroma. Near zero for a neutral ladder; the CRT phosphors run high.
    pub chroma: f32,
}

impl Ink {
    /// The ordinary case: a near-neutral ladder carrying the page's own
    /// temperature. Warm pages get warm greys, cool pages cool ones.
    pub fn neutral_for(page: (u8, u8, u8)) -> Self {
        let p = crate::oklch::from_srgb(page);
        Self {
            hue: p.h,
            chroma: (p.c * NEUTRAL_CHROMA_SHARE).min(NEUTRAL_CHROMA_CAP),
        }
    }

    /// The ladder a theme's existing `ink` already implies — hue and chroma
    /// read straight off it. This is how each preset is fitted to the ramp
    /// without moving: the theme keeps its own colour, and gains only the
    /// shared spacing.
    pub fn of(ink: (u8, u8, u8)) -> Self {
        let c = crate::oklch::from_srgb(ink);
        Self {
            hue: c.h,
            chroma: c.c,
        }
    }
}

/// How much of the page's chroma the neutral roles carry.
///
/// Zero would make every theme's greys identical and kill the warm/cool cast
/// that distinguishes SEPIA from AURORA; 1.0 would make body text as colourful
/// as the page, which reads as a tint rather than as ink. A third keeps the
/// temperature legible without the text looking dyed.
const NEUTRAL_CHROMA_SHARE: f32 = 0.33;

/// Upper bound on neutral chroma regardless of the page.
///
/// A strongly coloured page (the CRT phosphors) would otherwise push its ink
/// far enough into the hue to read as coloured text rather than as ink.
const NEUTRAL_CHROMA_CAP: f32 = 0.03;

/// The text ladder for one page colour.
#[derive(Clone, Copy, Debug)]
pub struct Ramp {
    page: (u8, u8, u8),
    toward: Toward,
    ink: Ink,
    house: House,
}

impl Ramp {
    /// Build the ladder for `page` with a near-neutral ink carrying the
    /// page's temperature — the ordinary case.
    pub fn for_page(page: (u8, u8, u8)) -> Self {
        Self::new(page, Ink::neutral_for(page), HOUSE)
    }

    /// Build the ladder for `page` on a declared [`Ink`] — what the CRT pool
    /// needs, and what fitting an existing preset uses.
    pub fn new(page: (u8, u8, u8), ink: Ink, house: House) -> Self {
        Self {
            page,
            toward: Toward::for_page(page),
            ink,
            house,
        }
    }

    /// The ramp a shipped preset already implies: its page, the hue and
    /// chroma of its own `ink`, and its pool's ladder. This is how each
    /// palette is fitted — it keeps its own colour and gains only the shared
    /// spacing.
    pub fn fitted(t: &crate::Theme) -> Self {
        let house = if t.is_tube() {
            // A white phosphor is not a coloured one; see HOUSE_CRT_WHITE.
            if crate::oklch::from_srgb(t.ink).c < 0.04 {
                HOUSE_CRT_WHITE
            } else {
                HOUSE_CRT
            }
        } else {
            HOUSE
        };
        Self::new(t.page_bg, Ink::of(t.ink), house)
    }

    /// The ladder this ramp derives against — paper/modern, coloured CRT, or
    /// white CRT. Exposed so tests can assert consistency *within* a ladder,
    /// which is the actual invariant; the three deliberately sit at different
    /// levels.
    pub fn house(&self) -> House {
        self.house
    }

    /// The colour hitting `target` contrast against the page.
    ///
    /// Hue and chroma are **not** constant down the ladder. A role's colour
    /// blends from the declared [`Ink`] at the top toward the page's own hue
    /// at the bottom, in proportion to how far down it sits. That is both what
    /// the hand-tuned palettes already do — MEADOW's border is greenish like
    /// its page, not blue like its ink — and the better design: something
    /// sitting at 2:1 against the page is nearly part of the page, and should
    /// look like it. Holding the ink's hue all the way down put a blue border
    /// on a green page.
    fn at(&self, target: f32) -> (u8, u8, u8) {
        let page = oklch::from_srgb(self.page);
        // 1.0 at the top of the ladder, 0.0 where a role would vanish into the
        // page. Contrast starts at 1.0 for identical colours, hence the -1.
        let depth = ((target - 1.0) / (self.house.ink - 1.0)).clamp(0.0, 1.0);
        // `page_pull` decides how much of that descent actually moves the hue.
        let t = 1.0 - (1.0 - depth) * self.house.page_pull;
        let hue = blend_hue(page.h, self.ink.hue, t);
        let chroma = page.c + (self.ink.chroma - page.c) * t;
        let solved = oklch::solve_for_contrast(self.page, hue, chroma, target, self.toward);
        self.cap(solved, hue, chroma)
    }

    /// Whether the lightness ceiling *bound* for a role at `target` contrast —
    /// that is, the ladder's ratio was unreachable without exceeding
    /// [`House::max_l`], so the role sits below its house level by design.
    ///
    /// Exact rather than "is the result near the cap": several themes land
    /// naturally within a thousandth of the ceiling without being limited by
    /// it, and an epsilon test cannot tell those apart.
    pub fn ceiling_bound(&self, target: f32) -> bool {
        let page = oklch::from_srgb(self.page);
        let depth = ((target - 1.0) / (self.house.ink - 1.0)).clamp(0.0, 1.0);
        let t = 1.0 - (1.0 - depth) * self.house.page_pull;
        let hue = blend_hue(page.h, self.ink.hue, t);
        let chroma = page.c + (self.ink.chroma - page.c) * t;
        let solved = oklch::solve_for_contrast(self.page, hue, chroma, target, self.toward);
        oklch::from_srgb(solved).l > self.house.max_l
    }

    /// Hold a role under the ladder's lightness ceiling. See [`House::max_l`].
    fn cap(&self, c: (u8, u8, u8), hue: f32, chroma: f32) -> (u8, u8, u8) {
        let l = oklch::from_srgb(c).l;
        if l <= self.house.max_l {
            return c;
        }
        oklch::Oklch::new(self.house.max_l, chroma, hue).to_srgb()
    }

    /// Primary chrome text.
    pub fn ink(&self) -> (u8, u8, u8) {
        self.at(self.house.ink)
    }
    /// Secondary body text.
    pub fn text_muted(&self) -> (u8, u8, u8) {
        self.at(self.house.text_muted)
    }
    /// Legend on an unfocused pane card.
    pub fn legend_off(&self) -> (u8, u8, u8) {
        self.at(self.house.legend_off)
    }
    /// Dim hint text on the input bar.
    pub fn dim(&self) -> (u8, u8, u8) {
        self.at(self.house.dim)
    }
    /// Hint text in the chat layout.
    pub fn hint_fg(&self) -> (u8, u8, u8) {
        self.at(self.house.hint_fg)
    }
    /// Input placeholder text.
    pub fn placeholder(&self) -> (u8, u8, u8) {
        self.at(self.house.placeholder)
    }
    /// Unfocused pane border.
    pub fn border_normal(&self) -> (u8, u8, u8) {
        self.at(self.house.border_normal)
    }
}

/// Interpolate `from` → `to` by `t`, the short way around the hue circle.
///
/// Going the long way would swing a warm ladder through green on its way to a
/// cool page, which shows up as a muddy mid-rung.
fn blend_hue(from: f32, to: f32, t: f32) -> f32 {
    let delta = (to - from + 540.0).rem_euclid(360.0) - 180.0;
    (from + delta * t).rem_euclid(360.0)
}

#[cfg(test)]
#[path = "ramp_tests.rs"]
mod tests;

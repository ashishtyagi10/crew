//! Glass: the frosted sheet each pane card sits on.
//!
//! Rather than adding a hand-tuned field to all thirteen `Theme` presets (and
//! to every future one), the glass look is *derived* from what a theme already
//! declares — `dark`, `crt`, `page_bg` and `ink`. Every theme therefore gets a
//! coherent sheet for free, and a new preset can never ship with glass missing.
//!
//! The three families want genuinely different treatment:
//!
//! * **dark** — glass catches light, so the sheet is *lighter* than the page
//!   and leans on a bright top hairline for its edge.
//! * **light** — a lighter-than-white sheet is invisible, so light themes get
//!   their depth from a whiter tint plus a real drop shadow.
//! * **CRT** — the tube post-process already adds bloom and scanlines; a full
//!   frost turns that to mud. CRT glass is deliberately the faintest of the
//!   three and skips the noise entirely (the tube supplies its own).
use crate::Theme;

/// How much glass to apply. `Off` disables the pass outright.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlassLevel {
    Off,
    Low,
    Medium,
    High,
}

impl GlassLevel {
    /// Multiplier applied to every alpha in [`GlassStyle`].
    pub fn scale(self) -> f32 {
        match self {
            GlassLevel::Off => 0.0,
            GlassLevel::Low => 0.55,
            GlassLevel::Medium => 1.0,
            GlassLevel::High => 1.6,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            GlassLevel::Off => "off",
            GlassLevel::Low => "low",
            GlassLevel::Medium => "medium",
            GlassLevel::High => "high",
        }
    }

    /// Parse a glass level name. Accepts the level names plus `on` as a
    /// friendly alias for the default strength.
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "off" | "none" => GlassLevel::Off,
            "low" | "subtle" => GlassLevel::Low,
            "on" | "medium" | "med" => GlassLevel::Medium,
            "high" | "strong" => GlassLevel::High,
            _ => return None,
        })
    }
}

/// Everything the glass pass needs to draw one pane card, in straight sRGB.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlassStyle {
    /// Frosted fill tint.
    pub tint: (u8, u8, u8),
    /// Fill opacity at the top and bottom edges. The vertical ramp between
    /// them is what reads as a sheet lit from above rather than a flat wash.
    pub alpha_top: f32,
    pub alpha_bottom: f32,
    /// Specular hairline just inside the top edge — the single cue that most
    /// says "glass", and the reason dark themes need no shadow to feel raised.
    pub highlight: (u8, u8, u8),
    pub highlight_alpha: f32,
    /// Soft drop shadow beneath the card.
    pub shadow_alpha: f32,
    /// Frost grain amplitude (0.0 = a clean sheet).
    pub noise: f32,
}

impl GlassStyle {
    /// Scale every alpha by `level`. `Off` yields a fully transparent style,
    /// which the renderer skips entirely.
    pub fn scaled(self, level: GlassLevel) -> Self {
        let k = level.scale();
        Self {
            alpha_top: (self.alpha_top * k).clamp(0.0, 1.0),
            alpha_bottom: (self.alpha_bottom * k).clamp(0.0, 1.0),
            highlight_alpha: (self.highlight_alpha * k).clamp(0.0, 1.0),
            shadow_alpha: (self.shadow_alpha * k).clamp(0.0, 1.0),
            // Noise rides the fill, so it scales too — but gently, or a High
            // sheet reads as sandpaper instead of frost.
            noise: self.noise * (0.5 + 0.5 * k),
            ..self
        }
    }

    /// Whether this style would draw anything at all.
    pub fn visible(self) -> bool {
        self.alpha_top > 0.001 || self.alpha_bottom > 0.001 || self.highlight_alpha > 0.001
    }
}

/// Blend `c` toward `target` by `t` (0 = unchanged, 1 = fully `target`).
fn mix(c: (u8, u8, u8), target: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let f = |a: u8, b: u8| -> u8 {
        let v = a as f32 + (b as f32 - a as f32) * t.clamp(0.0, 1.0);
        v.round().clamp(0.0, 255.0) as u8
    };
    (f(c.0, target.0), f(c.1, target.1), f(c.2, target.2))
}

/// The base (Medium-strength) glass for a theme.
pub fn style_for(t: &Theme) -> GlassStyle {
    if t.crt {
        // Phosphor tube: the sheet is a faint lift of the page toward the ink
        // colour, so the glass glows in the tube's own hue instead of graying
        // it. No noise — the CRT pass already grains the whole frame.
        return GlassStyle {
            tint: mix(t.page_bg, t.ink, 0.30),
            alpha_top: 0.10,
            alpha_bottom: 0.04,
            highlight: mix(t.page_bg, t.ink, 0.75),
            highlight_alpha: 0.14,
            shadow_alpha: 0.0,
            noise: 0.0,
        };
    }
    if t.dark {
        // Glass over a dark page reads as a lifted sheet: lighter than the
        // page, brightest at the top, with a near-white specular edge.
        GlassStyle {
            tint: mix(t.page_bg, (255, 255, 255), 0.42),
            alpha_top: 0.20,
            alpha_bottom: 0.09,
            highlight: (255, 255, 255),
            highlight_alpha: 0.22,
            shadow_alpha: 0.30,
            noise: 0.012,
        }
    } else {
        // A light page cannot get lighter, so the depth comes from a whiter,
        // more opaque sheet AND a real shadow — without the shadow a light
        // theme's glass is simply invisible.
        GlassStyle {
            tint: (255, 255, 255),
            alpha_top: 0.55,
            alpha_bottom: 0.30,
            highlight: (255, 255, 255),
            highlight_alpha: 0.60,
            shadow_alpha: 0.16,
            noise: 0.010,
        }
    }
}

/// Base glass for the currently active theme.
pub fn style() -> GlassStyle {
    style_for(crate::theme())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ALL_THEMES, CRT_GREEN, PAPER_DARK, PAPER_LIGHT};

    #[test]
    fn level_round_trips_and_accepts_aliases() {
        for l in [
            GlassLevel::Off,
            GlassLevel::Low,
            GlassLevel::Medium,
            GlassLevel::High,
        ] {
            assert_eq!(GlassLevel::parse(l.as_str()), Some(l));
        }
        assert_eq!(GlassLevel::parse("on"), Some(GlassLevel::Medium));
        assert_eq!(GlassLevel::parse("NONE"), Some(GlassLevel::Off));
        assert_eq!(GlassLevel::parse(" High "), Some(GlassLevel::High));
        assert_eq!(GlassLevel::parse("shiny"), None);
    }

    /// The whole point of deriving rather than declaring: no theme, present or
    /// future, can ship without glass.
    #[test]
    fn every_theme_gets_visible_glass() {
        for id in ALL_THEMES {
            let s = style_for(id.theme());
            assert!(
                s.scaled(GlassLevel::Medium).visible(),
                "{} has no visible glass",
                id.as_str()
            );
        }
    }

    #[test]
    fn off_draws_nothing() {
        for id in ALL_THEMES {
            assert!(!style_for(id.theme()).scaled(GlassLevel::Off).visible());
        }
    }

    /// Dark glass is a *lift*: the sheet must be lighter than the page it sits
    /// on, or the card reads as a hole rather than a pane of glass.
    #[test]
    fn dark_glass_lifts_off_the_page() {
        let s = style_for(&PAPER_DARK);
        let lum = |c: (u8, u8, u8)| c.0 as u32 + c.1 as u32 + c.2 as u32;
        assert!(
            lum(s.tint) > lum(PAPER_DARK.page_bg),
            "dark tint {:?} is not lighter than page {:?}",
            s.tint,
            PAPER_DARK.page_bg
        );
    }

    /// A light page can't get lighter, so light themes must earn their depth
    /// with a shadow. Without this the light glass is invisible.
    #[test]
    fn light_glass_has_a_shadow() {
        assert!(style_for(&PAPER_LIGHT).shadow_alpha > 0.0);
    }

    /// CRT stays faint and grain-free — the tube post-process supplies both
    /// bloom and noise, and doubling up turns the phosphor to mud.
    #[test]
    fn crt_glass_is_restrained() {
        let crt = style_for(&CRT_GREEN);
        let dark = style_for(&PAPER_DARK);
        assert!(
            crt.alpha_top < dark.alpha_top,
            "CRT glass should be fainter than plain dark glass"
        );
        assert_eq!(crt.noise, 0.0, "the tube already grains the frame");
        assert_eq!(crt.shadow_alpha, 0.0, "a tube has no drop shadows");
    }

    /// The top edge is always at least as opaque as the bottom — that ramp is
    /// what makes the sheet look lit from above.
    #[test]
    fn fill_is_brightest_at_the_top() {
        for id in ALL_THEMES {
            let s = style_for(id.theme());
            assert!(
                s.alpha_top >= s.alpha_bottom,
                "{} inverts the glass gradient",
                id.as_str()
            );
        }
    }

    #[test]
    fn level_scales_alpha_monotonically() {
        let base = style_for(&PAPER_DARK);
        let low = base.scaled(GlassLevel::Low).alpha_top;
        let med = base.scaled(GlassLevel::Medium).alpha_top;
        let high = base.scaled(GlassLevel::High).alpha_top;
        assert!(low < med && med < high, "{low} {med} {high}");
    }

    /// High strength must not push any alpha past opaque.
    #[test]
    fn high_never_exceeds_opaque() {
        for id in ALL_THEMES {
            let s = style_for(id.theme()).scaled(GlassLevel::High);
            for a in [
                s.alpha_top,
                s.alpha_bottom,
                s.highlight_alpha,
                s.shadow_alpha,
            ] {
                assert!((0.0..=1.0).contains(&a), "{} alpha {a}", id.as_str());
            }
        }
    }

    #[test]
    fn mix_interpolates_between_endpoints() {
        assert_eq!(mix((0, 0, 0), (255, 255, 255), 0.0), (0, 0, 0));
        assert_eq!(mix((0, 0, 0), (255, 255, 255), 1.0), (255, 255, 255));
        assert_eq!(mix((0, 0, 0), (200, 100, 50), 0.5), (100, 50, 25));
        // Out-of-range factors clamp rather than wrapping around.
        assert_eq!(mix((10, 10, 10), (200, 200, 200), -1.0), (10, 10, 10));
        assert_eq!(mix((10, 10, 10), (200, 200, 200), 2.0), (200, 200, 200));
    }
}

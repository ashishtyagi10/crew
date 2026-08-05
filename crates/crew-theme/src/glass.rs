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
//! * **CRT** — a holographic sheet: the pane is a luminous translucent panel
//!   of phosphor-tinted light (TRON light-trace, JARVIS HUD), *more* opaque
//!   than paper-dark's glass and lit from its own frame by an inner edge-glow.
//!   The old "faintest of the family" doctrine was repealed by the 2026-08-04
//!   holographic-overhaul goal — restraint made CRT read as a dark theme
//!   wearing scanlines, not a projection. What survives of that doctrine is
//!   what it got right: no drop shadow (a light construct casts none) and no
//!   frost grain (the tube post-process grains the whole frame itself).
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
    /// Inner edge-glow strength: how much the fill brightens toward the card
    /// border, as if the pane body were lit by its own frame. Zero on paper
    /// themes (sheets, not light constructs), so zero must reach the shader.
    pub edge_glow: f32,
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
            edge_glow: (self.edge_glow * k).clamp(0.0, 1.0),
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
    if t.crt.is_some() {
        // Holographic phosphor sheet: a strong lift of the page toward the ink
        // colour, MORE opaque than paper-dark's glass — the pane is a panel of
        // tinted light, not a whisper over the tube (goal 2026-08-04), and the
        // edge-glow makes the body read as lit by its own frame. Still no
        // noise (the CRT pass grains the whole frame) and no shadow (a light
        // construct casts none).
        return GlassStyle {
            tint: mix(t.page_bg, t.ink, 0.45),
            alpha_top: 0.26,
            alpha_bottom: 0.12,
            highlight: mix(t.page_bg, t.ink, 0.75),
            highlight_alpha: 0.30,
            shadow_alpha: 0.0,
            noise: 0.0,
            edge_glow: 0.35,
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
            edge_glow: 0.0,
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
            edge_glow: 0.0,
        }
    }
}

/// Base glass for the currently active theme.
pub fn style() -> GlassStyle {
    style_for(crate::theme())
}

#[cfg(test)]
#[path = "glass_tests.rs"]
mod tests;

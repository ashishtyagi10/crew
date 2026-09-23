//! Glass: the liquid-glass sheet each pane card sits on.
//!
//! Brought back 2026-09-23 on paper and modern pages ([`style_for`]); the
//! tubes stay flat. History, kept because the look below is built against it:
//! every family went flat on 2026-08-06. Paper went first (2026-08-06, morning): the
//! derived sheets' drop shadow read as a rendering bug on a light page, not
//! depth. CRT followed the same day: the holographic sheet — a ramped
//! phosphor fill with a specular hairline and inner edge-glow — read as a
//! drop shadow around every pane and made the cards look adrift on the page,
//! floating farther apart than the same grid on paper-dark. A tube differs
//! from paper-dark by its bloom, its heavier frame and its typeface — not by
//! depth.
//!
//! The `/glass` level scales it; `off` skips the pass outright.
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
    /// says "glass".
    pub highlight: (u8, u8, u8),
    pub highlight_alpha: f32,
    /// Soft two-layer shadow beneath the card (contact + ambient). Zero on
    /// the tubes: a CRT light construct casts none.
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

/// The base (Medium-strength) glass for a theme: LIQUID GLASS on every page
/// that is paper or modern, flat on the tubes.
///
/// The 2026-08-06 flat decree retired a sheet for two faults, and this look
/// is built around both rather than repeating them. The old sheet ran to the
/// cell edge — half a cell outside the frame's stroke — so its shadow drew a
/// second, misaligned box; the sheet now sits under the stroke (crew-render's
/// `stroke_centre`). And its one wide 14px shadow read as panes adrift; the
/// shadow now has a tight contact layer that says where the card rests.
///
/// Light pages: a white lens over the paper — brighter at the top — with a
/// white rim and a soft grey shadow. Dark pages: the faintest white lift, a
/// rim at a third of the light one (a bright rim on a dark page reads as a
/// neon outline), and a shadow strong enough to see on a near-black page.
/// A CRT tube is a light construct and casts nothing: its identity is bloom,
/// frame weight and typeface.
pub fn style_for(t: &Theme) -> GlassStyle {
    if t.is_tube() {
        return GlassStyle {
            tint: t.page_bg,
            alpha_top: 0.0,
            alpha_bottom: 0.0,
            highlight: t.page_bg,
            highlight_alpha: 0.0,
            shadow_alpha: 0.0,
            noise: 0.0,
            edge_glow: 0.0,
        };
    }
    let white = (255, 255, 255);
    match t.dark {
        false => GlassStyle {
            tint: white,
            alpha_top: 0.30,
            alpha_bottom: 0.14,
            highlight: white,
            highlight_alpha: 0.85,
            shadow_alpha: 0.12,
            noise: 0.0,
            edge_glow: 0.0,
        },
        true => GlassStyle {
            tint: white,
            alpha_top: 0.05,
            alpha_bottom: 0.015,
            highlight: white,
            highlight_alpha: 0.28,
            shadow_alpha: 0.45,
            noise: 0.0,
            edge_glow: 0.0,
        },
    }
}

/// Base glass for the currently active theme.
pub fn style() -> GlassStyle {
    style_for(crate::theme())
}

#[cfg(test)]
#[path = "glass_tests.rs"]
mod tests;

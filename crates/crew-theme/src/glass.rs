//! Glass: the (retired) frosted sheet each pane card sat on.
//!
//! Every family is flat now. Paper went first (2026-08-06, morning): the
//! derived sheets' drop shadow read as a rendering bug on a light page, not
//! depth. CRT followed the same day: the holographic sheet — a ramped
//! phosphor fill with a specular hairline and inner edge-glow — read as a
//! drop shadow around every pane and made the cards look adrift on the page,
//! floating farther apart than the same grid on paper-dark. A tube differs
//! from paper-dark by its bloom, its heavier frame and its typeface — not by
//! depth. The card's depth is its border and nothing else.
//!
//! The derivation (`style_for`) and the GPU plumbing stay: the shader, the
//! `/glass` level knob and the `GlassStyle` contract are the mechanism by
//! which a future look could bring a sheet back, and the renderer skips
//! invisible styles for free.
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
    /// Soft drop shadow beneath the card. Zero everywhere since 2026-08-06:
    /// paper themes are flat and a CRT light construct casts none. The
    /// plumbing stays for the shader's sake.
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

/// The base (Medium-strength) glass for a theme: flat for every family.
/// All-zero alphas make `visible()` false, so the renderer skips the cards
/// entirely — on paper because the sheet's shadow read as a misaligned
/// duplicate border, on CRT because the luminous sheet read as a drop shadow
/// that set the panes adrift (the tube's identity lives in bloom, border
/// weight and typeface instead).
pub fn style_for(t: &Theme) -> GlassStyle {
    GlassStyle {
        tint: t.page_bg,
        alpha_top: 0.0,
        alpha_bottom: 0.0,
        highlight: t.page_bg,
        highlight_alpha: 0.0,
        shadow_alpha: 0.0,
        noise: 0.0,
        edge_glow: 0.0,
    }
}

/// Base glass for the currently active theme.
pub fn style() -> GlassStyle {
    style_for(crate::theme())
}

#[cfg(test)]
#[path = "glass_tests.rs"]
mod tests;

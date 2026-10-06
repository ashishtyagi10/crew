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
    /// Soft two-layer shadow beneath the card (contact + ambient) — or, at
    /// `glow` 1, a halo of the sheet's own light (see `glow`).
    pub shadow_alpha: f32,
    /// Frost grain amplitude (0.0 = a clean sheet).
    pub noise: f32,
    /// Inner edge-glow strength: how much the fill brightens toward the card
    /// border, as if the pane body were lit by its own frame. Zero on paper
    /// themes (sheets, not light constructs), so zero must reach the shader.
    pub edge_glow: f32,
    /// The GLOSS: a broad specular reflection across the upper part of the
    /// sheet, in the highlight colour — the curved shine a thick, glossy
    /// slab of glass throws. Zero on paper and modern pages, whose sheets
    /// are frost, not gloss.
    pub gloss: f32,
    /// How far the shadow GLOWS instead of shading, `0.0..=1.0`: 0 is a black
    /// shadow (a sheet resting on paper), 1 a halo in the sheet's tint —
    /// light leaking out of a lit slab onto a dark page, which a black
    /// shadow on near-black could never show.
    pub glow: f32,
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
            gloss: (self.gloss * k).clamp(0.0, 1.0),
            ..self
        }
    }

    /// Whether this style would draw anything at all.
    pub fn visible(self) -> bool {
        self.alpha_top > 0.001
            || self.alpha_bottom > 0.001
            || self.highlight_alpha > 0.001
            || self.gloss > 0.001
    }
}

/// The base (Medium-strength) glass for a theme: LIQUID GLASS on every page.
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
/// Tubes: see [`tube_glass`].
pub fn style_for(t: &Theme) -> GlassStyle {
    if t.is_tube() {
        return tube_glass(t);
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
            gloss: 0.0,
            glow: 0.0,
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
            gloss: 0.0,
            glow: 0.0,
        },
    }
}

/// A tube's glass (2026-10-05, the "terminal running in glass" goal): a
/// slab of glass lit from within by its own phosphor, where the flat-tube
/// decree had left phosphor text on a bare black page.
///
/// Everything is the tube's own colour — its focused frame's phosphor — so
/// green glass for a green tube and orchid for violet with no table to keep
/// in step. The body is a clearly visible tint, deeper at the bottom; a
/// near-white phosphor rim and a broad GLOSS across the upper face say it is
/// glossy and thick; the edge-glow brightens the band inside the frame the
/// way a slab's edges catch light; and its shadow GLOWS — a halo of the
/// phosphor round the card, light leaking out of the glass onto the page.
///
/// The strengths are held by the text on top: green ink on green glass
/// loses contrast fastest, and at the High level the corner where the body,
/// the edge glow and the gloss stack still keeps every tube's terminal text
/// above 7:1 (`tube_text_reads_on_its_glass`). What says "glass" beyond
/// that is spent where no text sits — the rim and the halo.
pub fn tube_glass(t: &Theme) -> GlassStyle {
    let p = t.border_focused;
    let toward_white = |c: u8| (f32::from(c) + (255.0 - f32::from(c)) * 0.7).round() as u8;
    GlassStyle {
        tint: p,
        alpha_top: 0.04,
        alpha_bottom: 0.012,
        highlight: (toward_white(p.0), toward_white(p.1), toward_white(p.2)),
        highlight_alpha: 0.7,
        shadow_alpha: 0.16,
        noise: 0.0,
        edge_glow: 0.05,
        gloss: 0.10,
        glow: 1.0,
    }
}

/// Base glass for the currently active theme.
pub fn style() -> GlassStyle {
    style_for(crate::theme())
}

#[cfg(test)]
#[path = "glass_tests.rs"]
mod tests;

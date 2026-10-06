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
    /// A raster ETCHED into the glass body: fine horizontal lines every few
    /// pixels, adding up to this much fill alpha. The tubes' old scanlines,
    /// moved off the window and into the panel, under the text — zero on
    /// clear glass.
    pub etch: f32,
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
            etch: (self.etch * k).clamp(0.0, 1.0),
            ..self
        }
    }

    /// This style with what lies UNDER the text scaled by `k`: the body's
    /// fill, its lit edge, the gloss and the etch all spend the text's
    /// contrast, so when the OS asks for more contrast they give it back by
    /// the same factor the page's wash does (`contrast::effect_scale`). The
    /// rim and the shadow or halo stay: no text sits on them, and they are
    /// what still says the pane is a sheet. `k == 1.0` returns the style
    /// untouched.
    pub fn quieted(self, k: f32) -> Self {
        if k == 1.0 {
            return self;
        }
        Self {
            alpha_top: self.alpha_top * k,
            alpha_bottom: self.alpha_bottom * k,
            edge_glow: self.edge_glow * k,
            gloss: self.gloss * k,
            etch: self.etch * k,
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
            etch: 0.0,
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
            etch: 0.0,
        },
    }
}

/// A tube's glass: the panes of a terminal running in glass (the 2026-10-05
/// goal), over a window that is itself sheer — the desktop frosted behind it
/// (crew-app's `tubesheer`, 2026-10-06: "just the glass and borders … almost
/// frosted glass, no black, like transparent").
///
/// So the body is FROST: a thin milky sheet of the phosphor run most of the
/// way to white, laid on lightly with a frost grain — the window server's
/// blur does the frosting, the sheet only says where the pane is. It was
/// dark SMOKE first, thick enough to hold the text off any wallpaper, and it
/// read as a dark window (the user, the same day: "still not transparent
/// enough, I said frosty glass, they are still dark"). Everything else is
/// the phosphor's own light: a near-white rim, a broad GLOSS across the
/// upper face, a fine raster ETCHED into the hot phosphors' glass, and a
/// shadow that GLOWS — a halo of the phosphor round the card (the shader
/// takes the milk back out of the frost's colour for it).
///
/// Held by the text on it over a dark desktop: `tube_text_reads_on_its_glass`
/// keeps every tube's terminal text above 7:1 on the frost at the High
/// level, under the gloss. A bright wallpaper costs the text contrast — the
/// user's taste: transparency first.
pub fn tube_glass(t: &Theme) -> GlassStyle {
    let p = t.border_focused;
    let toward =
        |a: u8, b: u8, k: f32| (f32::from(a) + (f32::from(b) - f32::from(a)) * k).round() as u8;
    let pale = |k: f32| {
        (
            toward(p.0, 255, k),
            toward(p.1, 255, k),
            toward(p.2, 255, k),
        )
    };
    GlassStyle {
        tint: pale(0.6),
        alpha_top: 0.08,
        alpha_bottom: 0.04,
        highlight: pale(0.7),
        highlight_alpha: 0.7,
        shadow_alpha: 0.16,
        noise: 0.035,
        edge_glow: 0.0,
        gloss: 0.07,
        glow: 1.0,
        // The tube's raster, if it keeps one, lives in its glass.
        etch: t.crt.map_or(0.0, |c| c.etch),
    }
}

/// Base glass for the currently active theme — quieted when the OS asks for
/// more contrast (see [`GlassStyle::quieted`]).
pub fn style() -> GlassStyle {
    style_for(crate::theme()).quieted(crate::contrast::effect_scale())
}

#[cfg(test)]
#[path = "glass_tests.rs"]
mod tests;

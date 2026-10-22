//! Holographic glass: the neon theme's dark glassmorphism (the user,
//! 2026-10-22: "dark glassmorphism theme with neon accents (electric blue,
//! violet, cyan) … glowing borders … fine grid lines … animated particles …
//! holographic effects … like a high-tech control system").
//!
//! Three layers, each drawn by crew-render, each kept off the words:
//! - a GRID behind the panes ([`backdrop`]): fine lines in the accent on a
//!   near-black page, brighter every fourth, with sparks drifting up it while
//!   a pane works — the panes float over it, and it shows through their glass;
//! - each pane a slab of blue-smoked glass ([`holo_glass`]) whose frame's
//!   light bleeds a little way into it, with a cyan-white rim and a halo of
//!   its own blue round it;
//! - the rim and the frame-light running cyan to violet across the card, the
//!   way a hologram's foil turns.
//!
//! [`crate::LiquidStyle::holo`] says how much of it a theme wears; 0 is plain
//! liquid glass, and every theme but neon's is 0.
use crate::{GlassStyle, LiquidStyle, Theme};

/// Neon's glass: a near-black window that lets a little of the desktop
/// through, and panes of thinner glass over it — so the grid on the page,
/// not the wallpaper, is what each pane is seen against.
pub(crate) const HOLO_LIQUID: LiquidStyle = LiquidStyle {
    window: 0.84,
    body: 0.42,
    desktop_blur: 24.0,
    holo: 1.0,
};

fn mix(a: (u8, u8, u8), b: (u8, u8, u8), k: f32) -> (u8, u8, u8) {
    let m = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * k).round() as u8;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// The slab: the page smoked toward the accent's electric blue, a cyan-white
/// rim, a gloss, the frame's light bleeding in (`edge_glow`, which the
/// shader runs cyan to violet across the card) and a halo of the smoke's own
/// blue round the card (`glow` 1) in place of a shadow — a shadow on
/// near-black draws nothing. `k` is [`LiquidStyle::holo`].
pub fn holo_glass(t: &Theme, k: f32) -> GlassStyle {
    GlassStyle {
        tint: mix(t.page_bg, t.accent_default, 0.13),
        alpha_top: 0.58,
        alpha_bottom: 0.70,
        highlight: mix(t.border_focused, (255, 255, 255), 0.45),
        highlight_alpha: 0.95,
        shadow_alpha: 0.08 + 0.10 * k,
        noise: 0.0,
        edge_glow: 0.22 * k,
        gloss: 0.12 * k,
        glow: 1.0,
        etch: 0.0,
    }
}

/// The hues of [`tag_rung`]: cyan, the electric blue, indigo and violet.
const TAG_HUES: [f32; 4] = [205.0, 240.0, 275.0, 305.0];

/// [`tag_rung`]'s three lightnesses, each with the chroma the gamut allows
/// it: a pale blue cannot be a saturated one.
const TAG_TONES: [(f32, f32); 3] = [(0.88, 0.09), (0.76, 0.13), (0.64, 0.16)];

/// A holographic page's signature colours (crew-theme `tagcolor`): what a
/// pane's legend, an agent's name and an `@project` tag are drawn in. The
/// shared pool borrows the terminal's twelve colours — red, yellow and green
/// among them — so on neon a pane titled `far` wore salmon; here the twelve
/// are four hues from cyan to violet at three lightnesses, every pair still
/// a visible step apart.
pub(crate) fn tag_rung(slot: usize) -> (u8, u8, u8) {
    let h = TAG_HUES[slot % TAG_HUES.len()];
    let (l, c) = TAG_TONES[(slot / TAG_HUES.len()) % TAG_TONES.len()];
    crate::oklch::Oklch::new(l, c, h).to_srgb()
}

/// What the grid behind the panes is drawn in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Backdrop {
    /// The grid's lines: the accent.
    pub line: (u8, u8, u8),
    /// The two kinds of spark: the focused frame's cyan and the violet.
    pub spark: (u8, u8, u8),
    pub spark2: (u8, u8, u8),
    /// [`LiquidStyle::holo`]: how strongly all of it is drawn.
    pub strength: f32,
}

/// `t`'s grid, or `None` on every theme that has none.
pub fn backdrop(t: &Theme) -> Option<Backdrop> {
    let l = t.liquid.filter(|l| l.holo > 0.0)?;
    Some(Backdrop {
        line: t.accent_default,
        spark: t.border_focused,
        spark2: t.broadcast,
        strength: l.holo,
    })
}

#[cfg(test)]
#[path = "holo_tests.rs"]
mod tests;

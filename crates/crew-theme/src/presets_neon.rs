//! NEON: a holographic control room (the user,
//! 2026-10-22: "an ultra-modern futuristic dashboard … dark glassmorphism
//! theme with neon accents (electric blue, violet, cyan) … glowing borders,
//! fine grid lines, animated particles … holographic effects … high
//! contrast"). It began the same day as a neon scoreboard (yellow, pink and
//! green on indigo, from a video); the redesign kept the name and moved it
//! into the brief's three neons. It had a light page too until the user
//! found `neon` serving it by day ("neon doesn't have to be light color,
//! keep it default dark", 2026-10-22): neon is one dark page now, whatever
//! the appearance or the clock.
//!
//! The page is holographic glass ([`crate::holo`]): a fine grid in the
//! electric blue behind blue-smoked panes, each with a cyan-white rim, its
//! frame's light bleeding into it and a halo of blue round it. The text
//! ladder and the ANSI slots are derived like every other palette's
//! ([`crate::ramp`], [`crate::ansi`]); what the preset declares is its page,
//! its ink's hue, and the neon roles.

use crate::{CrtStyle, ModernStyle, Theme};

/// Neon, dark: a near-black blue-violet room, bright white words, the
/// electric blue for the accent and the grid, cyan for the focused frame, a
/// bluer cyan for progress (it runs along that frame, so it has to show on
/// it), violet for the broadcast.
pub static NEON_DARK: Theme = Theme {
    page_bg: (7, 8, 24),
    ink: (228, 232, 242),
    text_muted: (190, 195, 210),
    term_fg: (228, 232, 242),
    term_bg: (7, 8, 24),
    border_normal: (62, 66, 85),
    border_focused: (64, 226, 255),
    border_thickness: 2.0,
    legend_off: (135, 140, 158),
    accent_default: (92, 162, 255),
    status_fg: (64, 226, 255),
    broadcast: (178, 112, 255),
    activity: (107, 198, 255),
    bell: (255, 120, 150),
    dim: (117, 121, 140),
    placeholder: (109, 113, 132),
    hint_fg: (118, 123, 142),
    find_hl_bg: (30, 50, 120),
    ansi: [
        (95, 97, 105),   // 0  black
        (252, 151, 173), // 1  red
        (130, 197, 123), // 2  green
        (213, 176, 78),  // 3  yellow
        (140, 181, 255), // 4  blue
        (204, 163, 247), // 5  magenta
        (44, 198, 220),  // 6  cyan
        (212, 213, 223), // 7  white
        (131, 133, 142), // 8  bright black
        (255, 183, 197), // 9  bright red
        (150, 218, 142), // 10 bright green
        (234, 196, 99),  // 11 bright yellow
        (172, 202, 255), // 12 bright blue
        (220, 188, 255), // 13 bright magenta
        (75, 219, 241),  // 14 bright cyan
        (234, 236, 246), // 15 bright white
    ],
    dark: true,
    // The bloom is the neon: every bright stroke — the focused frame, the
    // rims, the grid's major lines — throws a soft glow; the shade is the
    // glass's text shadow.
    crt: Some(CrtStyle {
        glow: 0.7,
        glow_radius: 14.0,
        ..crate::presets_glass_clear::CLEAR_BLOOM
    }),
    // The focus ring runs cyan to violet through the electric blue.
    modern: Some(ModernStyle {
        pole_a: (64, 226, 255),
        pole_b: (178, 112, 255),
        drift_ms: 6_000,
    }),
    liquid: Some(crate::holo::HOLO_LIQUID),
    syntax: crate::syntaxslot::NEON_DARK,
    selection: None,
};

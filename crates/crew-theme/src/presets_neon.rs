//! NEON presets: a holographic control room, dark and light (the user,
//! 2026-10-22: "an ultra-modern futuristic dashboard … dark glassmorphism
//! theme with neon accents (electric blue, violet, cyan) … glowing borders,
//! fine grid lines, animated particles … holographic effects … high
//! contrast"). It began the same day as a neon scoreboard (yellow, pink and
//! green on indigo, from a video); the redesign kept the name and the
//! dark/light pair and moved it into the brief's three neons.
//!
//! The dark page is holographic glass ([`crate::holo`]): a fine grid in the
//! electric blue behind blue-smoked panes, each with a cyan-white rim, its
//! frame's light bleeding into it and a halo of blue round it. The light page
//! is print, the same three neons deepened until they read on white. The text
//! ladder and the ANSI slots are derived like every other palette's
//! ([`crate::ramp`], [`crate::ansi`]); what each preset declares is its page,
//! its ink's hue, and the neon roles.

use crate::{CrtStyle, ModernStyle, Theme};

/// Neon, dark: a near-black blue-violet room, bright white words, the
/// electric blue for the accent and the grid, cyan for the focused frame and
/// progress, violet for the broadcast.
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
    activity: (90, 200, 255),
    bell: (255, 120, 150),
    dim: (117, 121, 140),
    placeholder: (109, 113, 132),
    hint_fg: (118, 123, 142),
    find_hl_bg: (30, 50, 120),
    ansi: [
        (95, 97, 105),   // 0  black
        (254, 153, 141), // 1  red
        (116, 198, 132), // 2  green
        (218, 174, 79),  // 3  yellow
        (124, 184, 255), // 4  blue
        (223, 158, 230), // 5  magenta
        (37, 200, 206),  // 6  cyan
        (212, 213, 223), // 7  white
        (131, 133, 142), // 8  bright black
        (255, 185, 175), // 9  bright red
        (136, 219, 151), // 10 bright green
        (239, 194, 100), // 11 bright yellow
        (162, 204, 255), // 12 bright blue
        (244, 178, 251), // 13 bright magenta
        (70, 221, 227),  // 14 bright cyan
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
};

/// Neon, light: the same room printed on a cool white page — the grid and
/// the glass belong to the dark page. The focused frame and the accent are
/// the electric blue, the progress the cyan and the broadcast the violet,
/// each deepened until it reads on white, and the words are a deep navy.
pub static NEON_LIGHT: Theme = Theme {
    page_bg: (245, 247, 255),
    ink: (19, 23, 58),
    text_muted: (48, 53, 80),
    term_fg: (19, 23, 58),
    term_bg: (245, 247, 255),
    border_normal: (141, 143, 156),
    border_focused: (24, 92, 240),
    border_thickness: 2.5,
    legend_off: (91, 95, 113),
    accent_default: (24, 92, 240),
    status_fg: (0, 118, 160),
    broadcast: (112, 48, 220),
    activity: (0, 112, 150),
    bell: (184, 40, 90),
    dim: (108, 112, 127),
    placeholder: (117, 120, 134),
    hint_fg: (107, 110, 126),
    find_hl_bg: (160, 192, 254),
    ansi: [
        (36, 35, 37),   // 0  black
        (144, 67, 59),  // 1  red
        (35, 104, 53),  // 2  green
        (116, 85, 0),   // 3  yellow
        (41, 92, 149),  // 4  blue
        (123, 71, 129), // 5  magenta
        (0, 101, 104),  // 6  cyan
        (72, 72, 75),   // 7  white
        (98, 97, 100),  // 8  bright black
        (126, 50, 43),  // 9  bright red
        (11, 86, 36),   // 10 bright green
        (96, 70, 0),    // 11 bright yellow
        (23, 75, 131),  // 12 bright blue
        (106, 55, 112), // 13 bright magenta
        (0, 83, 86),    // 14 bright cyan
        (37, 36, 39),   // 15 bright white
    ],
    dark: false,
    crt: None,
    modern: None,
    liquid: None,
};

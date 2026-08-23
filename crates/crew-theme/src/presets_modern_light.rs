//! MODERN-family presets, the LIGHT half (first pair): the same Gemini /
//! Codex look with the lights on — near-white pages carrying a faint hue
//! cast, deep slate ink, and the family's two saturated poles driving the
//! focused ring, the gradient wash and the dot lattice.
//!
//! The dark half's poles are pastels (they have to glow on a near-black
//! page); on white they would vanish, so every light palette drives the same
//! hues DEEP — the ring reads as ink-strength colour and the wash/dots tint
//! the page downward instead of lifting it. `dots` and `wash` are pulled a
//! little under the dark family's for the same reason: darkening a white page
//! reads stronger than lifting a black one. Palettes validated against the
//! `contrast_thresholds` suite at design time (scripted WCAG sweep,
//! 2026-08-13) before a line of this file existed.

use crate::{CrtStyle, ModernStyle, Theme};

/// **Daybreak** (Aurora with the lights on): a cool near-white page under
/// deep slate ink, with Aurora's blue→violet polar sky driven to full
/// saturation — the polar sky at midday rather than midnight.
pub static DAYBREAK: Theme = Theme {
    page_bg: (250, 251, 254),
    ink: (22, 29, 48),
    text_muted: (51, 56, 71),
    term_fg: (17, 24, 39),
    term_bg: (250, 251, 254),
    border_normal: (179, 180, 185),
    border_focused: (37, 99, 235),
    border_thickness: 3.5,
    legend_off: (94, 98, 107),
    accent_default: (37, 99, 235),
    status_fg: (30, 64, 175),
    broadcast: (190, 24, 93),
    activity: (29, 78, 216),
    bell: (180, 83, 9),
    dim: (112, 114, 122),
    placeholder: (120, 122, 130),
    hint_fg: (111, 113, 121),
    find_hl_bg: (191, 219, 254),
    ansi: [
        (38, 38, 39),    // 0  black
        (147, 69, 62),   // 1  red
        (36, 105, 54),   // 2  green
        (119, 87, 0),    // 3  yellow
        (44, 94, 151),   // 4  blue
        (125, 73, 131),  // 5  magenta
        (0, 103, 107),   // 6  cyan
        (74, 74, 75),    // 7  white
        (100, 100, 102), // 8  bright black
        (128, 52, 46),   // 9  bright red
        (14, 88, 38),    // 10 bright green
        (99, 72, 0),     // 11 bright yellow
        (27, 77, 133),   // 12 bright blue
        (108, 57, 114),  // 13 bright magenta
        (0, 85, 89),     // 14 bright cyan
        (39, 39, 40),    // 15 bright white
    ],
    dark: false,
    grain: 0.0,
    crt: Some(CrtStyle {
        curvature: 0.0,
        scanline: 0.0,
        glow: 0.35,
        glow_radius: 12.0,
        corner: 0.0,
        flicker: 0.03,
    }),
    modern: Some(ModernStyle {
        pole_a: (37, 99, 235),
        pole_b: (124, 58, 237),
        drift_ms: 6_000,
        dots: 0.16,
        wash: 0.12,
    }),
};

/// **Blossom** (Nebula with the lights on): a warm white page with the
/// violet→rose sweep of Nebula's dusk, deepened until it reads on paper.
pub static BLOSSOM: Theme = Theme {
    page_bg: (253, 250, 252),
    ink: (22, 29, 46),
    text_muted: (57, 55, 69),
    term_fg: (17, 24, 39),
    term_bg: (253, 250, 252),
    border_normal: (182, 179, 182),
    border_focused: (147, 51, 234),
    border_thickness: 3.5,
    legend_off: (101, 96, 104),
    accent_default: (147, 51, 234),
    status_fg: (126, 34, 206),
    broadcast: (190, 24, 93),
    activity: (162, 28, 175),
    bell: (180, 83, 9),
    dim: (118, 113, 119),
    placeholder: (126, 121, 127),
    hint_fg: (117, 111, 118),
    find_hl_bg: (243, 208, 240),
    ansi: [
        (38, 37, 38),   // 0  black
        (147, 69, 62),  // 1  red
        (36, 105, 54),  // 2  green
        (119, 87, 0),   // 3  yellow
        (44, 94, 151),  // 4  blue
        (125, 73, 131), // 5  magenta
        (0, 103, 107),  // 6  cyan
        (75, 74, 75),   // 7  white
        (101, 99, 100), // 8  bright black
        (128, 52, 46),  // 9  bright red
        (14, 88, 38),   // 10 bright green
        (99, 72, 0),    // 11 bright yellow
        (27, 77, 133),  // 12 bright blue
        (108, 57, 114), // 13 bright magenta
        (0, 85, 89),    // 14 bright cyan
        (40, 39, 40),   // 15 bright white
    ],
    dark: false,
    grain: 0.0,
    crt: Some(CrtStyle {
        curvature: 0.0,
        scanline: 0.0,
        glow: 0.4,
        glow_radius: 13.0,
        corner: 0.0,
        flicker: 0.03,
    }),
    modern: Some(ModernStyle {
        pole_a: (147, 51, 234),
        pole_b: (219, 39, 119),
        drift_ms: 6_000,
        dots: 0.16,
        wash: 0.12,
    }),
};

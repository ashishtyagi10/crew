//! The `light` mode's rose glass (2026-10-09: every mode see-through, light
//! "a smoked clear glass with white words"). Blossom was Nebula with the
//! lights on — a warm white page with a violet→rose sweep; it is now a
//! cherry-blossom smoke over the desktop with white words, the sweep lighting
//! its focus ring.

use crate::{CrtStyle, ModernStyle, Theme};

/// **Blossom**: a cherry-blossom smoke, pink accents, white words.
pub static BLOSSOM: Theme = Theme {
    page_bg: (36, 22, 30),
    ink: (247, 239, 243),
    text_muted: (216, 205, 211),
    term_fg: (247, 239, 243),
    term_bg: (36, 22, 30),
    border_normal: (87, 71, 79),
    border_focused: (226, 232, 246),
    border_thickness: 3.5,
    legend_off: (162, 147, 155),
    accent_default: (255, 170, 200),
    status_fg: (255, 214, 10),
    broadcast: (210, 140, 255),
    activity: (100, 210, 255),
    bell: (255, 118, 110),
    dim: (143, 128, 136),
    placeholder: (134, 119, 127),
    hint_fg: (144, 129, 137),
    find_hl_bg: (46, 57, 126),
    ansi: [
        (111, 104, 107), // 0  black
        (255, 172, 161), // 1  red
        (129, 211, 145), // 2  green
        (230, 187, 92),  // 3  yellow
        (147, 197, 255), // 4  blue
        (236, 170, 243), // 5  magenta
        (58, 212, 218),  // 6  cyan
        (234, 226, 230), // 7  white
        (149, 142, 145), // 8  bright black
        (255, 203, 195), // 9  bright red
        (149, 232, 165), // 10 bright green
        (251, 207, 113), // 11 bright yellow
        (184, 217, 255), // 12 bright blue
        (250, 196, 255), // 13 bright magenta
        (86, 233, 239),  // 14 bright cyan
        (255, 250, 252), // 15 bright white
    ],
    dark: true,
    grain: 0.0,
    crt: Some(CrtStyle {
        // Not a tube: the bloom lights the focus ring, the shade is the
        // glass's text shadow.
        scanline: 0.0,
        glow: 0.4,
        glow_radius: 13.0,
        flicker: 0.03,
        core: 0.0,
        etch: 0.0,
        shade: CrtStyle::GLASS_SHADE,
    }),
    modern: Some(ModernStyle {
        pole_a: (255, 170, 200),
        pole_b: (200, 160, 255),
        drift_ms: 6_000,
        // No wallpaper: the desktop is the background.
        dots: 0.0,
        wash: 0.0,
    }),
    liquid: Some(crate::presets_glass_clear::LIGHT_LIQUID),
};

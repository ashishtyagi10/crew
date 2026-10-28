//! PURPLE: one palette, `amethyst` (2026-10-28, the user: "add one purple
//! color theme"). It is its look's whole pool, the way neon is, so `/theme
//! purple` serves it and the `Ctrl+Shift+L` cycle and `auto` reach it.
//!
//! The page itself is purple. `nebula` was already violet, but a near-black
//! one: there the purple lives in the accent, and any purple accent on a
//! page like that came out within Δ 0.05 of it (the reason Dracula and
//! Catppuccin stayed out of `classic`). A royal-purple page, as saturated as
//! the house ladder allows, is what puts amethyst past
//! `no_two_palettes_are_near_duplicates`, with a lavender accent paler
//! than nebula's orchid.
//!
//! Smoked glass like the rest ([`DARK_LIQUID`]). The text ladder, the ANSI
//! slots and the search wash are what [`crate::ramp`], [`crate::ansi`] and
//! [`crate::highlight`] derive.

use crate::presets_glass_clear::DARK_LIQUID;
use crate::{CrtStyle, ModernStyle, Theme};

/// **Amethyst**: a royal-purple page, white words with a lilac cast,
/// lavender for the accent, a vivid violet frame, and the ring running
/// violet to magenta.
pub static AMETHYST: Theme = Theme {
    page_bg: (34, 6, 58),
    ink: (245, 239, 254),
    text_muted: (210, 198, 229),
    term_fg: (245, 239, 254),
    term_bg: (34, 6, 58),
    border_normal: (85, 62, 114),
    border_focused: (186, 120, 255),
    border_thickness: 3.5,
    legend_off: (158, 139, 184),
    accent_default: (225, 185, 255),
    status_fg: (228, 206, 255),
    broadcast: (255, 130, 200),
    activity: (196, 170, 255),
    bell: (255, 184, 150),
    dim: (140, 119, 168),
    placeholder: (132, 111, 160),
    hint_fg: (141, 121, 169),
    find_hl_bg: (80, 60, 118),
    ansi: [
        (105, 101, 113), // 0  black
        (255, 165, 153), // 1  red
        (125, 207, 141), // 2  green
        (226, 183, 88),  // 3  yellow
        (140, 192, 255), // 4  blue
        (232, 166, 239), // 5  magenta
        (52, 208, 214),  // 6  cyan
        (226, 221, 235), // 7  white
        (143, 138, 150), // 8  bright black
        (255, 196, 188), // 9  bright red
        (145, 228, 161), // 10 bright green
        (247, 203, 109), // 11 bright yellow
        (177, 212, 255), // 12 bright blue
        (249, 190, 255), // 13 bright magenta
        (81, 229, 235),  // 14 bright cyan
        (249, 244, 255), // 15 bright white
    ],
    dark: true,
    crt: Some(CrtStyle {
        scanline: 0.0,
        glow: 0.85,
        glow_radius: 14.0,
        flicker: 0.0,
        core: 0.0,
        etch: 0.0,
        shade: CrtStyle::GLASS_SHADE,
    }),
    modern: Some(ModernStyle {
        pole_a: (168, 85, 247),
        pole_b: (236, 72, 153),
        drift_ms: 6_000,
    }),
    liquid: Some(DARK_LIQUID),
};

#[cfg(test)]
#[path = "purple_tests.rs"]
mod tests;

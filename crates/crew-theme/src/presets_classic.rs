//! CLASSIC: editor palettes people already know by name (2026-10-28, the
//! user: "add 5 new theme for crew"). Each keeps its own page hue, its
//! signature colours (frame, accent, the focus ring's poles and the status
//! roles) and its ink's hue. The rest is derived like every other palette's:
//! the text ladder ([`crate::ramp`]), the ANSI slots ([`crate::ansi`]) and
//! the search wash ([`crate::highlight`]). The ANSI slots take the scheme's
//! own six hues ([`crate::ansihue`]) at crew's contrast, so shell output and
//! syntax colours read as the scheme's.
//!
//! The five were picked by measurement. On crew's deep pages, with the
//! house's near-white ink, Dracula, Catppuccin and Nord came out within
//! Δ 0.05 of `nebula`, `neon` or `glass-night` whatever accent they took, so
//! they would be the near-duplicates `no_two_palettes_are_near_duplicates`
//! keeps out. These five each own a hue nothing else on the roster has.
//!
//! They are smoked glass like `nebula` ([`DARK_LIQUID`]). The cool two live
//! here and the warm three in `presets_classic2.rs`, so neither file passes
//! 200 lines.

use crate::presets_glass_clear::DARK_LIQUID;
use crate::{CrtStyle, ModernStyle, Theme};

/// The bloom every classic palette starts from: the white-text glass's
/// shadow, a halo for the ring and no flicker (none of these is a tube).
/// Each palette sets its own glow, so no two share a tuning.
pub(crate) const CLASSIC_BLOOM: CrtStyle = CrtStyle {
    scanline: 0.0,
    glow: 0.7,
    glow_radius: 12.0,
    flicker: 0.0,
    core: 0.0,
    etch: 0.0,
    shade: CrtStyle::GLASS_SHADE,
};

/// **Tokyo Night**: the deep navy of a city after dark (`#1a1b26`), the
/// neon-sign blue for the frame and the ring running blue to magenta, its
/// cyan for the accent (its blue sat within Δ 0.05 of `neon`'s).
pub static TOKYO_NIGHT: Theme = Theme {
    page_bg: (26, 27, 38),
    ink: (238, 241, 255),
    text_muted: (206, 209, 223),
    term_fg: (238, 241, 255),
    term_bg: (26, 27, 38),
    border_normal: (74, 76, 88),
    border_focused: (122, 162, 247),
    border_thickness: 3.5,
    legend_off: (149, 151, 166),
    accent_default: (125, 207, 255),
    status_fg: (192, 211, 251),
    broadcast: (187, 154, 247),
    activity: (158, 206, 106),
    bell: (255, 158, 100),
    dim: (130, 132, 145),
    placeholder: (122, 124, 137),
    hint_fg: (132, 134, 147),
    find_hl_bg: (32, 50, 140),
    ansi: [
        (106, 107, 112), // 0  black
        (255, 172, 183), // 1  red
        (165, 207, 120), // 2  green
        (242, 184, 98),  // 3  yellow
        (165, 195, 255), // 4  blue
        (212, 181, 255), // 5  magenta
        (103, 206, 255), // 6  cyan
        (228, 229, 236), // 7  white
        (144, 145, 150), // 8  bright black
        (255, 204, 209), // 9  bright red
        (185, 228, 140), // 10 bright green
        (255, 207, 140), // 11 bright yellow
        (196, 216, 255), // 12 bright blue
        (227, 208, 255), // 13 bright magenta
        (160, 223, 255), // 14 bright cyan
        (253, 253, 255), // 15 bright white
    ],
    dark: true,
    crt: Some(CrtStyle {
        glow: 0.75,
        ..CLASSIC_BLOOM
    }),
    modern: Some(ModernStyle {
        pole_a: (122, 162, 247),
        pole_b: (187, 154, 247),
        drift_ms: 6_000,
    }),
    liquid: Some(DARK_LIQUID),
    syntax: crate::syntaxslot::TOKYO_NIGHT,
    selection: Some((0x28, 0x34, 0x57)),
};

/// **Solarized** (dark): the deep teal of `base03`, deepened a step so the
/// house ladder has room above it, words in its `base1` lifted to the
/// house's white, yellow for the accent, blue for the frame and the ring
/// running blue to cyan.
pub static SOLARIZED: Theme = Theme {
    page_bg: (0, 29, 37),
    ink: (230, 245, 245),
    text_muted: (190, 211, 213),
    term_fg: (230, 245, 245),
    term_bg: (0, 29, 37),
    border_normal: (51, 80, 88),
    border_focused: (38, 139, 210),
    border_thickness: 3.5,
    legend_off: (128, 155, 160),
    accent_default: (203, 172, 75),
    status_fg: (176, 213, 239),
    broadcast: (143, 147, 210),
    activity: (62, 206, 194),
    bell: (238, 153, 151),
    dim: (108, 136, 142),
    placeholder: (99, 128, 134),
    hint_fg: (110, 138, 144),
    find_hl_bg: (71, 79, 50),
    ansi: [
        (96, 108, 111),  // 0  black
        (255, 171, 155), // 1  red
        (182, 201, 105), // 2  green
        (228, 187, 91),  // 3  yellow
        (131, 199, 255), // 4  blue
        (255, 166, 204), // 5  magenta
        (64, 213, 201),  // 6  cyan
        (216, 230, 234), // 7  white
        (133, 146, 149), // 8  bright black
        (255, 202, 191), // 9  bright red
        (202, 222, 125), // 10 bright green
        (249, 207, 112), // 11 bright yellow
        (174, 218, 255), // 12 bright blue
        (255, 200, 222), // 13 bright magenta
        (91, 234, 221),  // 14 bright cyan
        (244, 252, 255), // 15 bright white
    ],
    dark: true,
    crt: Some(CrtStyle {
        glow: 0.6,
        ..CLASSIC_BLOOM
    }),
    modern: Some(ModernStyle {
        pole_a: (38, 139, 210),
        pole_b: (42, 161, 152),
        drift_ms: 6_000,
    }),
    liquid: Some(DARK_LIQUID),
    syntax: crate::syntaxslot::SOLARIZED,
    selection: Some((0x07, 0x36, 0x42)),
};

#[cfg(test)]
#[path = "classic_tests.rs"]
mod tests;

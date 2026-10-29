//! The `light` mode's rose glass (2026-10-09: every mode see-through, light
//! "a smoked clear glass with white words"). Blossom was Nebula with the
//! lights on — a warm white page with a violet→rose sweep; it is now a
//! cherry-blossom smoke over the desktop with white words, the sweep lighting
//! its focus ring.

use crate::{CrtStyle, ModernStyle, Theme};

/// **Blossom**: a cherry-blossom smoke, pink accents, white words. Since
/// 2026-10-29 the smoke is a wine rose (it was a near-grey within Δ 0.052
/// of `glass-clear`, the closest pair on the roster), the focused frame a
/// blush white, progress a fresh leaf green, and code in spring colours
/// ([`crate::ansihue`]): sakura keywords, leaf strings, apricot types, sky
/// calls, coral numbers.
pub static BLOSSOM: Theme = Theme {
    page_bg: (46, 15, 31),
    ink: (247, 239, 243),
    text_muted: (222, 202, 211),
    term_fg: (247, 239, 243),
    term_bg: (46, 15, 31),
    border_normal: (100, 65, 81),
    border_focused: (252, 228, 238),
    border_thickness: 3.5,
    legend_off: (173, 142, 156),
    accent_default: (255, 170, 200),
    status_fg: (184, 231, 147),
    broadcast: (210, 140, 255),
    activity: (100, 210, 255),
    bell: (255, 118, 110),
    dim: (155, 123, 137),
    placeholder: (146, 114, 129),
    hint_fg: (156, 124, 138),
    find_hl_bg: (122, 32, 71),
    ansi: [
        (116, 102, 107), // 0  black
        (255, 171, 163), // 1  red
        (149, 208, 129), // 2  green
        (243, 181, 98),  // 3  yellow
        (131, 199, 255), // 4  blue
        (253, 165, 220), // 5  magenta
        (74, 213, 191),  // 6  cyan
        (240, 223, 230), // 7  white
        (154, 140, 146), // 8  bright black
        (255, 203, 195), // 9  bright red
        (169, 229, 149), // 10 bright green
        (255, 205, 141), // 11 bright yellow
        (174, 218, 255), // 12 bright blue
        (255, 198, 232), // 13 bright magenta
        (99, 234, 211),  // 14 bright cyan
        (255, 250, 252), // 15 bright white
    ],
    dark: true,
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
    }),
    liquid: Some(crate::presets_glass_clear::LIGHT_LIQUID),
    syntax: crate::syntaxslot::BLOSSOM,
    selection: None,
};

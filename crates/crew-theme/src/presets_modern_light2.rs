//! MODERN-family presets, the LIGHT half (second pair) — see
//! `presets_modern_light.rs` for the family conventions. Split the same way
//! the dark half is, to keep both files small.

use crate::{CrtStyle, ModernStyle, Theme};

/// **Meadow** (Graphene with the lights on): the neutral engineering page,
/// its mint→cyan light deepened into emerald→teal so the weave and the ring
/// read as colour on white.
pub static MEADOW: Theme = Theme {
    page_bg: (249, 251, 249),
    ink: (22, 29, 44),
    text_muted: (44, 58, 65),
    term_fg: (17, 24, 39),
    term_bg: (249, 251, 249),
    border_normal: (177, 180, 178),
    border_focused: (13, 148, 136),
    border_thickness: 3.5,
    legend_off: (90, 99, 98),
    accent_default: (13, 148, 136),
    status_fg: (15, 118, 110),
    broadcast: (190, 24, 93),
    activity: (5, 150, 105),
    bell: (180, 83, 9),
    dim: (108, 116, 113),
    placeholder: (117, 124, 121),
    hint_fg: (107, 114, 112),
    find_hl_bg: (183, 231, 214),
    ansi: [
        (55, 65, 81),    // 0  black
        (185, 28, 28),   // 1  red
        (21, 128, 61),   // 2  green
        (161, 98, 7),    // 3  yellow
        (29, 78, 216),   // 4  blue
        (147, 51, 234),  // 5  magenta
        (12, 98, 120),   // 6  cyan
        (75, 85, 99),    // 7  white
        (107, 114, 128), // 8  bright black
        (220, 38, 38),   // 9  bright red
        (22, 163, 74),   // 10 bright green
        (180, 111, 8),   // 11 bright yellow
        (37, 99, 235),   // 12 bright blue
        (168, 85, 247),  // 13 bright magenta
        (11, 128, 156),  // 14 bright cyan
        (31, 41, 55),    // 15 bright white
    ],
    dark: false,
    grain: 0.0,
    crt: Some(CrtStyle {
        curvature: 0.0,
        scanline: 0.0,
        glow: 0.3,
        glow_radius: 12.0,
        corner: 0.0,
        flicker: 0.03,
    }),
    modern: Some(ModernStyle {
        pole_a: (5, 150, 105),
        pole_b: (14, 116, 144),
        drift_ms: 7_000,
        dots: 0.16,
        wash: 0.12,
    }),
};

/// **Cirrus** (Cobalt with the lights on): the coolest page of the four,
/// swept blue→cyan — high cloud in daylight.
pub static CIRRUS: Theme = Theme {
    page_bg: (247, 250, 253),
    ink: (21, 28, 46),
    text_muted: (48, 56, 70),
    term_fg: (17, 24, 39),
    term_bg: (247, 250, 253),
    border_normal: (176, 179, 184),
    border_focused: (29, 78, 216),
    border_thickness: 3.5,
    legend_off: (91, 97, 106),
    accent_default: (13, 106, 130),
    status_fg: (30, 64, 175),
    broadcast: (190, 24, 93),
    activity: (14, 116, 144),
    bell: (180, 83, 9),
    dim: (109, 114, 121),
    placeholder: (117, 122, 129),
    hint_fg: (107, 113, 120),
    find_hl_bg: (186, 224, 240),
    ansi: [
        (55, 65, 81),    // 0  black
        (185, 28, 28),   // 1  red
        (21, 128, 61),   // 2  green
        (161, 98, 7),    // 3  yellow
        (29, 78, 216),   // 4  blue
        (147, 51, 234),  // 5  magenta
        (12, 98, 120),   // 6  cyan
        (75, 85, 99),    // 7  white
        (107, 114, 128), // 8  bright black
        (220, 38, 38),   // 9  bright red
        (22, 163, 74),   // 10 bright green
        (180, 111, 8),   // 11 bright yellow
        (37, 99, 235),   // 12 bright blue
        (168, 85, 247),  // 13 bright magenta
        (11, 128, 156),  // 14 bright cyan
        (31, 41, 55),    // 15 bright white
    ],
    dark: false,
    grain: 0.0,
    crt: Some(CrtStyle {
        curvature: 0.0,
        scanline: 0.0,
        glow: 0.32,
        glow_radius: 13.0,
        corner: 0.0,
        flicker: 0.03,
    }),
    modern: Some(ModernStyle {
        pole_a: (29, 78, 216),
        pole_b: (13, 106, 130),
        drift_ms: 6_000,
        dots: 0.16,
        wash: 0.12,
    }),
};

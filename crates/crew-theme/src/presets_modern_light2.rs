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
        (37, 38, 37),   // 0  black
        (146, 68, 61),  // 1  red
        (36, 105, 54),  // 2  green
        (119, 87, 0),   // 3  yellow
        (44, 94, 151),  // 4  blue
        (125, 73, 131), // 5  magenta
        (0, 103, 107),  // 6  cyan
        (74, 74, 74),   // 7  white
        (99, 100, 99),  // 8  bright black
        (127, 51, 45),  // 9  bright red
        (14, 88, 38),   // 10 bright green
        (99, 72, 0),    // 11 bright yellow
        (27, 77, 133),  // 12 bright blue
        (108, 57, 114), // 13 bright magenta
        (0, 85, 89),    // 14 bright cyan
        (39, 39, 39),   // 15 bright white
    ],
    dark: false,
    grain: 0.0,
    crt: Some(CrtStyle {
        scanline: 0.0,
        glow: 0.3,
        glow_radius: 12.0,
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
        (37, 37, 38),   // 0  black
        (146, 68, 61),  // 1  red
        (35, 104, 53),  // 2  green
        (117, 87, 0),   // 3  yellow
        (43, 93, 150),  // 4  blue
        (124, 73, 130), // 5  magenta
        (0, 102, 106),  // 6  cyan
        (73, 73, 74),   // 7  white
        (98, 99, 101),  // 8  bright black
        (127, 51, 45),  // 9  bright red
        (13, 87, 37),   // 10 bright green
        (97, 72, 0),    // 11 bright yellow
        (26, 76, 132),  // 12 bright blue
        (107, 57, 113), // 13 bright magenta
        (0, 84, 88),    // 14 bright cyan
        (38, 39, 39),   // 15 bright white
    ],
    dark: false,
    grain: 0.0,
    crt: Some(CrtStyle {
        scanline: 0.0,
        glow: 0.32,
        glow_radius: 13.0,
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

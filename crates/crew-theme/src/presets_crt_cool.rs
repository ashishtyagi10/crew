//! CRT-family presets, the cool half: the violet, blue and paperwhite
//! phosphors — the cool end of the tube family, split from `presets_crt.rs`
//! to keep both files under the line cap. Where the hot phosphors (green,
//! amber) run coarse rasters and jumpy flicker, these run wide, smooth bloom
//! (violet, blue) or a crisp steady raster (paperwhite): projected light
//! rather than a driven gun. Frame weight and glow follow the flat-tube
//! decree (2026-08-06) — see `presets_crt.rs`.

use crate::{CrtStyle, Theme};

/// **Neon violet phosphor** (Tron-grid): ultraviolet orchid traced over a
/// deep cool near-black tube — the fourth phosphor, glowing purple.
/// Style: the JARVIS orchid HUD — faint scanlines, a wide luminous halo,
/// and a gentle flicker: projected light, not a raster gun.
pub static CRT_VIOLET: Theme = Theme {
    page_bg: (5, 2, 8),
    ink: (237, 193, 254),
    text_muted: (194, 152, 210),
    term_fg: (232, 170, 255),
    term_bg: (5, 2, 8),
    // Unfocused borders sit back (focus-led hierarchy, as in paper-dark).
    border_normal: (100, 61, 114),
    border_focused: (235, 150, 255),
    border_thickness: 3.5,
    legend_off: (164, 122, 179),
    accent_default: (245, 170, 255),
    status_fg: (245, 185, 250),
    broadcast: (255, 150, 200),
    activity: (205, 130, 255),
    bell: (255, 190, 240),
    dim: (119, 80, 133),
    placeholder: (138, 98, 153),
    hint_fg: (152, 112, 168),
    find_hl_bg: (60, 25, 85),
    ansi: [
        (97, 94, 99),    // 0  black
        (159, 105, 179), // 1  red
        (173, 118, 193), // 2  green
        (187, 132, 207), // 3  yellow
        (201, 145, 222), // 4  blue
        (215, 159, 237), // 5  magenta
        (230, 173, 251), // 6  cyan
        (212, 209, 215), // 7  white
        (133, 130, 135), // 8  bright black
        (179, 124, 199), // 9  bright red
        (193, 137, 214), // 10 bright green
        (207, 151, 228), // 11 bright yellow
        (222, 165, 243), // 12 bright blue
        (234, 181, 255), // 13 bright magenta
        (241, 202, 255), // 14 bright cyan
        (234, 231, 237), // 15 bright white
    ],
    dark: true,
    grain: 1.2,
    crt: Some(CrtStyle {
        scanline: 0.12,
        glow: 1.05,
        glow_radius: 10.0,
        flicker: 0.05,
    }),
    modern: None,
};

/// **Neon blue phosphor** (Tron light-cycle grid): electric edge-glow cyan
/// traced over a deep near-black tube — the coolest of the four grids, page
/// and phosphor alike.
/// Style: the TRON light-trace — scanlines almost gone, the widest and
/// strongest bloom of the family, and the steadiest glow: every stroke a
/// light-cycle trail.
pub static CRT_BLUE: Theme = Theme {
    page_bg: (1, 4, 8),
    ink: (27, 229, 255),
    text_muted: (0, 184, 205),
    term_fg: (0, 229, 255),
    term_bg: (1, 4, 8),
    // Unfocused borders sit back (focus-led hierarchy, as in paper-dark).
    border_normal: (0, 85, 95),
    border_focused: (0, 225, 255),
    border_thickness: 3.5,
    legend_off: (0, 152, 170),
    accent_default: (90, 255, 255),
    status_fg: (150, 230, 255),
    broadcast: (170, 180, 255),
    activity: (0, 220, 255),
    bell: (170, 220, 255),
    dim: (0, 105, 117),
    placeholder: (0, 125, 140),
    hint_fg: (0, 140, 156),
    find_hl_bg: (10, 45, 75),
    ansi: [
        (92, 95, 99),    // 0  black
        (0, 154, 172),   // 1  red
        (0, 169, 189),   // 2  green
        (6, 184, 205),   // 3  yellow
        (45, 198, 220),  // 4  blue
        (67, 212, 234),  // 5  magenta
        (85, 227, 249),  // 6  cyan
        (206, 211, 214), // 7  white
        (128, 132, 135), // 8  bright black
        (47, 174, 192),  // 9  bright red
        (50, 189, 209),  // 10 bright green
        (55, 204, 226),  // 11 bright yellow
        (75, 219, 241),  // 12 bright blue
        (94, 233, 255),  // 13 bright magenta
        (161, 241, 255), // 14 bright cyan
        (228, 233, 237), // 15 bright white
    ],
    dark: true,
    grain: 1.2,
    crt: Some(CrtStyle {
        scanline: 0.10,
        glow: 1.1,
        glow_radius: 12.0,
        flicker: 0.04,
    }),
    modern: None,
};

/// **Paperwhite phosphor** (P4, the early-Macintosh/VT420 white tube):
/// near-white ink with the faintest blue-gray cast on a true black tube —
/// the fifth phosphor, and the only one that reads as print rather than
/// neon; its ANSI palette is brightness tiers with faint hue tilts, like
/// crt-green's single-gun look in white.
/// Style: the crispest tube of the family — fine scanlines, a modest cool
/// halo, and the steadiest beam (P4 is a fast, low-persistence phosphor):
/// a page of light, not a driven raster.
pub static CRT_PAPERWHITE: Theme = Theme {
    page_bg: (4, 5, 7),
    ink: (225, 230, 237),
    text_muted: (188, 193, 199),
    term_fg: (235, 240, 248),
    term_bg: (4, 5, 7),
    // Unfocused borders sit back (focus-led hierarchy, as in paper-dark).
    border_normal: (73, 78, 83),
    border_focused: (230, 238, 248),
    border_thickness: 3.5,
    legend_off: (134, 139, 145),
    accent_default: (200, 220, 255),
    status_fg: (215, 225, 240),
    broadcast: (205, 190, 230),
    activity: (180, 210, 245),
    bell: (225, 230, 245),
    dim: (116, 121, 127),
    placeholder: (108, 113, 119),
    hint_fg: (117, 122, 128),
    find_hl_bg: (45, 55, 75),
    ansi: [
        (95, 95, 97),    // 0  black
        (251, 151, 139), // 1  red
        (114, 196, 130), // 2  green
        (215, 172, 76),  // 3  yellow
        (120, 182, 254), // 4  blue
        (221, 156, 228), // 5  magenta
        (30, 197, 203),  // 6  cyan
        (210, 211, 212), // 7  white
        (131, 132, 133), // 8  bright black
        (255, 181, 171), // 9  bright red
        (134, 217, 149), // 10 bright green
        (236, 192, 97),  // 11 bright yellow
        (157, 202, 255), // 12 bright blue
        (242, 176, 249), // 13 bright magenta
        (66, 218, 224),  // 14 bright cyan
        (232, 234, 235), // 15 bright white
    ],
    dark: true,
    grain: 1.2,
    crt: Some(CrtStyle {
        scanline: 0.14,
        glow: 0.7,
        glow_radius: 8.0,
        flicker: 0.03,
    }),
    modern: None,
};

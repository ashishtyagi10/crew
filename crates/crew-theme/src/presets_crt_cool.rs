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
        (55, 35, 75),    // 0  black
        (255, 140, 200), // 1  red
        (190, 150, 255), // 2  green
        (235, 180, 255), // 3  yellow
        (160, 140, 255), // 4  blue
        (230, 140, 255), // 5  magenta
        (200, 160, 255), // 6  cyan
        (230, 200, 250), // 7  white
        (140, 95, 175),  // 8  bright black
        (255, 160, 220), // 9  bright red
        (210, 170, 255), // 10 bright green
        (245, 200, 255), // 11 bright yellow
        (180, 160, 255), // 12 bright blue
        (240, 160, 255), // 13 bright magenta
        (215, 180, 255), // 14 bright cyan
        (245, 225, 255), // 15 bright white
    ],
    dark: true,
    grain: 1.2,
    crt: Some(CrtStyle {
        curvature: 0.0,
        scanline: 0.12,
        glow: 1.05,
        glow_radius: 10.0,
        corner: 0.0,
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
        (20, 50, 75),    // 0  black
        (150, 170, 255), // 1  red
        (0, 255, 220),   // 2  green
        (140, 220, 255), // 3  yellow
        (60, 160, 255),  // 4  blue
        (150, 150, 255), // 5  magenta
        (0, 240, 255),   // 6  cyan
        (170, 225, 255), // 7  white
        (0, 120, 170),   // 8  bright black
        (180, 190, 255), // 9  bright red
        (60, 255, 235),  // 10 bright green
        (170, 235, 255), // 11 bright yellow
        (90, 190, 255),  // 12 bright blue
        (180, 170, 255), // 13 bright magenta
        (110, 250, 255), // 14 bright cyan
        (200, 240, 255), // 15 bright white
    ],
    dark: true,
    grain: 1.2,
    crt: Some(CrtStyle {
        curvature: 0.0,
        scanline: 0.10,
        glow: 1.1,
        glow_radius: 12.0,
        corner: 0.0,
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
        (55, 60, 70),    // 0  black
        (255, 180, 175), // 1  red
        (185, 235, 200), // 2  green
        (240, 230, 190), // 3  yellow
        (170, 200, 250), // 4  blue
        (225, 190, 240), // 5  magenta
        (185, 225, 245), // 6  cyan
        (215, 222, 232), // 7  white
        (130, 140, 155), // 8  bright black
        (255, 200, 195), // 9  bright red
        (205, 245, 215), // 10 bright green
        (250, 240, 205), // 11 bright yellow
        (190, 215, 252), // 12 bright blue
        (238, 205, 250), // 13 bright magenta
        (205, 238, 250), // 14 bright cyan
        (245, 248, 252), // 15 bright white
    ],
    dark: true,
    grain: 1.2,
    crt: Some(CrtStyle {
        curvature: 0.0,
        scanline: 0.14,
        glow: 0.7,
        glow_radius: 8.0,
        corner: 0.0,
        flicker: 0.03,
    }),
    modern: None,
};

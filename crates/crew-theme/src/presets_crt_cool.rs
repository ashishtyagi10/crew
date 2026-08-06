//! CRT-family presets, the cool half: the violet and blue phosphors — the
//! HUD end of the tube family, split from `presets_crt.rs` to keep both
//! files under the line cap. Where the hot phosphors (green, amber) run
//! coarse rasters and jumpy flicker, these two run wide, smooth bloom:
//! projected light rather than a driven gun. Frame weight and glow follow
//! the flat-tube decree (2026-08-06) — see `presets_crt.rs`.

use crate::{CrtStyle, Theme};

/// **Neon violet phosphor** (Tron-grid): ultraviolet orchid traced over a
/// deep cool near-black tube — the fourth phosphor, glowing purple.
/// Style: the JARVIS orchid HUD — faint scanlines, a wide luminous halo,
/// and a gentle flicker: projected light, not a raster gun.
pub static CRT_VIOLET: Theme = Theme {
    page_bg: (5, 2, 8),
    ink: (232, 170, 255),
    text_muted: (205, 140, 235),
    term_fg: (232, 170, 255),
    term_bg: (5, 2, 8),
    // Unfocused borders sit back (focus-led hierarchy, as in paper-dark).
    border_normal: (88, 50, 110),
    border_focused: (235, 150, 255),
    border_thickness: 3.5,
    legend_off: (170, 115, 200),
    accent_default: (245, 170, 255),
    status_fg: (245, 185, 250),
    broadcast: (255, 150, 200),
    activity: (205, 130, 255),
    bell: (255, 190, 240),
    dim: (120, 78, 145),
    placeholder: (135, 88, 162),
    hint_fg: (150, 100, 180),
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
};

/// **Neon blue phosphor** (Tron light-cycle grid): electric edge-glow cyan
/// traced over a deep near-black tube — the coolest of the four grids, page
/// and phosphor alike.
/// Style: the TRON light-trace — scanlines almost gone, the widest and
/// strongest bloom of the family, and the steadiest glow: every stroke a
/// light-cycle trail.
pub static CRT_BLUE: Theme = Theme {
    page_bg: (1, 4, 8),
    ink: (0, 229, 255),
    text_muted: (0, 182, 214),
    term_fg: (0, 229, 255),
    term_bg: (1, 4, 8),
    // Unfocused borders sit back (focus-led hierarchy, as in paper-dark).
    border_normal: (0, 78, 110),
    border_focused: (0, 225, 255),
    border_thickness: 3.5,
    legend_off: (0, 145, 180),
    accent_default: (90, 255, 255),
    status_fg: (150, 230, 255),
    broadcast: (170, 180, 255),
    activity: (0, 220, 255),
    bell: (170, 220, 255),
    dim: (0, 105, 140),
    placeholder: (0, 122, 155),
    hint_fg: (0, 138, 172),
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
};

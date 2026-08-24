//! CRT-family presets, the cool half: the violet, blue and paperwhite
//! phosphors — the cool end of the tube family, split from `presets_crt.rs`
//! to keep both files under the line cap. Where the hot phosphors (green,
//! amber) run coarse rasters and jumpy flicker, these run wide, smooth bloom
//! (violet, blue) or a crisp steady raster (paperwhite): projected light
//! rather than a driven gun. Frame weight and glow follow the flat-tube
//! decree (2026-08-06) — see `presets_crt.rs`.

use crate::{CrtStyle, ModernStyle, Theme};

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
    modern: Some(ModernStyle {
        pole_a: (0, 169, 189),
        pole_b: (85, 227, 249),
        drift_ms: 6_000,
        dots: 0.10,
        wash: 0.10,
    }),
};

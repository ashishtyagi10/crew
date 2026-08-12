//! MODERN-family presets, the aurora half: the look of today's AI apps
//! (Gemini, Codex) rather than yesterday's terminals — deep neutral pages,
//! one confident accent hue per palette, soft wide bloom with every retro
//! knob (curvature, scanlines, bezel, grain) at zero, and a gradient
//! light-ring on the focused frame (`ModernStyle`). Split from
//! `presets_modern2.rs` to keep both files small. Palettes validated against
//! the `contrast_thresholds` suite at design time (scripted WCAG sweep,
//! 2026-08-10).

use crate::{CrtStyle, ModernStyle, Theme};

/// **Aurora** (the Gemini look): a deep cool charcoal page under crisp
/// off-white ink, with the blue→violet gradient of a polar sky as the accent
/// pair. Glow is a soft halo — light behind glass, not phosphor.
pub static AURORA: Theme = Theme {
    page_bg: (15, 17, 23),
    ink: (232, 235, 243),
    text_muted: (196, 203, 217),
    term_fg: (232, 235, 243),
    term_bg: (15, 17, 23),
    border_normal: (56, 62, 80),
    border_focused: (138, 180, 248),
    border_thickness: 3.5,
    legend_off: (146, 156, 178),
    accent_default: (138, 180, 248),
    status_fg: (178, 198, 245),
    broadcast: (240, 148, 200),
    activity: (140, 182, 250),
    bell: (255, 200, 130),
    dim: (120, 130, 152),
    placeholder: (112, 122, 145),
    hint_fg: (118, 128, 150),
    find_hl_bg: (46, 62, 96),
    ansi: [
        (52, 58, 74),    // 0  black
        (242, 139, 130), // 1  red
        (129, 201, 149), // 2  green
        (253, 214, 99),  // 3  yellow
        (138, 180, 248), // 4  blue
        (197, 138, 249), // 5  magenta
        (120, 213, 227), // 6  cyan
        (216, 222, 233), // 7  white
        (124, 134, 156), // 8  bright black
        (250, 165, 158), // 9  bright red
        (155, 215, 170), // 10 bright green
        (255, 226, 130), // 11 bright yellow
        (170, 199, 250), // 12 bright blue
        (215, 168, 252), // 13 bright magenta
        (154, 225, 237), // 14 bright cyan
        (238, 242, 249), // 15 bright white
    ],
    dark: true,
    grain: 0.0,
    crt: Some(CrtStyle {
        curvature: 0.0,
        scanline: 0.0,
        glow: 0.8,
        glow_radius: 12.0,
        corner: 0.0,
        flicker: 0.03,
    }),
    modern: Some(ModernStyle {
        pole_a: (110, 168, 254),
        pole_b: (197, 138, 249),
        drift_ms: 6_000,
        dots: 0.20,
    }),
};

/// **Nebula**: aurora's dusk sibling — a violet-cast near-black page with the
/// gradient sliding from orchid to rose. The most saturated of the family,
/// with the widest halo.
pub static NEBULA: Theme = Theme {
    page_bg: (19, 15, 26),
    ink: (238, 232, 247),
    text_muted: (206, 196, 222),
    term_fg: (238, 232, 247),
    term_bg: (19, 15, 26),
    border_normal: (66, 56, 88),
    border_focused: (197, 138, 249),
    border_thickness: 3.5,
    legend_off: (158, 146, 182),
    accent_default: (202, 148, 250),
    status_fg: (215, 185, 250),
    broadcast: (255, 150, 200),
    activity: (206, 152, 250),
    bell: (255, 196, 160),
    dim: (132, 120, 156),
    placeholder: (124, 112, 148),
    hint_fg: (130, 118, 154),
    find_hl_bg: (62, 44, 94),
    ansi: [
        (58, 50, 78),    // 0  black
        (250, 140, 168), // 1  red
        (150, 210, 160), // 2  green
        (250, 208, 120), // 3  yellow
        (150, 170, 252), // 4  blue
        (212, 148, 252), // 5  magenta
        (140, 208, 232), // 6  cyan
        (222, 214, 236), // 7  white
        (134, 122, 158), // 8  bright black
        (255, 168, 190), // 9  bright red
        (172, 224, 182), // 10 bright green
        (255, 222, 150), // 11 bright yellow
        (178, 192, 253), // 12 bright blue
        (226, 176, 253), // 13 bright magenta
        (168, 222, 240), // 14 bright cyan
        (243, 238, 250), // 15 bright white
    ],
    dark: true,
    grain: 0.0,
    crt: Some(CrtStyle {
        curvature: 0.0,
        scanline: 0.0,
        glow: 0.9,
        glow_radius: 13.0,
        corner: 0.0,
        flicker: 0.03,
    }),
    modern: Some(ModernStyle {
        pole_a: (197, 138, 249),
        pole_b: (244, 143, 177),
        drift_ms: 6_000,
        dots: 0.20,
    }),
};

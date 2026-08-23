//! MODERN-family presets, the aurora half: the look of today's AI apps
//! (Gemini, Codex) rather than yesterday's terminals — deep neutral pages,
//! one confident accent hue per palette, soft wide bloom with every retro
//! knob (curvature, scanlines, bezel, grain) at zero, and a gradient
//! light-ring on the focused frame (`ModernStyle`). Split from
//! `presets_modern2.rs` to keep both files small. Palettes validated against
//! the `contrast_thresholds` suite at design time (scripted WCAG sweep,
//! 2026-08-10).

use crate::{CrtStyle, ModernStyle, Theme};

/// **Nebula**: aurora's dusk sibling — a violet-cast near-black page with the
/// gradient sliding from orchid to rose. The most saturated of the family,
/// with the widest halo.
pub static NEBULA: Theme = Theme {
    page_bg: (19, 15, 26),
    ink: (241, 235, 250),
    text_muted: (202, 197, 212),
    term_fg: (238, 232, 247),
    term_bg: (19, 15, 26),
    border_normal: (73, 68, 81),
    border_focused: (197, 138, 249),
    border_thickness: 3.5,
    legend_off: (147, 142, 156),
    accent_default: (202, 148, 250),
    status_fg: (215, 185, 250),
    broadcast: (255, 150, 200),
    activity: (206, 152, 250),
    bell: (255, 196, 160),
    dim: (128, 123, 137),
    placeholder: (120, 115, 129),
    hint_fg: (130, 125, 139),
    find_hl_bg: (62, 44, 94),
    ansi: [
        (101, 100, 105), // 0  black
        (255, 160, 148), // 1  red
        (121, 203, 137), // 2  green
        (222, 179, 84),  // 3  yellow
        (132, 189, 255), // 4  blue
        (228, 162, 235), // 5  magenta
        (45, 204, 210),  // 6  cyan
        (220, 218, 224), // 7  white
        (138, 137, 143), // 8  bright black
        (255, 192, 183), // 9  bright red
        (141, 224, 157), // 10 bright green
        (243, 199, 105), // 11 bright yellow
        (169, 209, 255), // 12 bright blue
        (248, 183, 255), // 13 bright magenta
        (76, 225, 231),  // 14 bright cyan
        (243, 241, 247), // 15 bright white
    ],
    dark: true,
    grain: 0.0,
    crt: Some(CrtStyle {
        scanline: 0.0,
        glow: 0.9,
        glow_radius: 13.0,
        flicker: 0.03,
    }),
    modern: Some(ModernStyle {
        pole_a: (197, 138, 249),
        pole_b: (244, 143, 177),
        drift_ms: 6_000,
        dots: 0.20,
        wash: 0.15,
    }),
};

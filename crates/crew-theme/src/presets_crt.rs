//! CRT-family presets, the hot half: the green and amber phosphors. Each
//! preset carries its own `CrtStyle` — the four tubes no longer share one
//! set of global post-process knobs, so these two run coarse, jittery
//! rasters while the cool pair (`presets_crt_cool.rs`: violet, blue) runs
//! wide, smooth, HUD-calm bloom.
//!
//! All four tubes run a 3.5px frame — heavier than any paper preset — since
//! the flat-tube decree (2026-08-06): the glass sheet is retired, so border
//! weight, bloom and typeface are the whole of what says "tube" over paper.

use crate::{CrtStyle, Theme};

/// **Neon green phosphor** (P1, Tron-grid): hot saturated green traced over
/// a deep cool near-black tube, with a monochrome-green ANSI palette
/// (brightness tiers, faint hue tilts) for that single-gun terminal look.
/// The paper-grain pass reads as a subtle glow off the grid lines.
/// Style: the hottest raster of the four — heavy scanlines, a strong but
/// tight bloom, and the jumpiest streaming flicker: a P1 tube driven hard.
pub static CRT_GREEN: Theme = Theme {
    page_bg: (2, 6, 5),
    ink: (0, 255, 102),
    text_muted: (0, 204, 82),
    term_fg: (0, 255, 102),
    term_bg: (2, 6, 5),
    // Unfocused borders sit back (matching paper-dark's focus-led hierarchy)
    // so the bright phosphor frame alone says which pane is live.
    border_normal: (0, 88, 42),
    border_focused: (0, 255, 120),
    border_thickness: 3.5,
    legend_off: (0, 160, 70),
    accent_default: (30, 255, 140),
    status_fg: (190, 255, 80),
    broadcast: (150, 255, 150),
    activity: (0, 255, 110),
    bell: (200, 255, 90),
    dim: (0, 110, 55),
    placeholder: (0, 135, 60),
    hint_fg: (0, 150, 66),
    find_hl_bg: (10, 70, 30),
    ansi: [
        (10, 45, 20),    // 0  black
        (170, 255, 70),  // 1  red
        (0, 255, 102),   // 2  green
        (200, 255, 80),  // 3  yellow
        (0, 230, 170),   // 4  blue
        (130, 255, 150), // 5  magenta
        (0, 255, 200),   // 6  cyan
        (170, 255, 190), // 7  white
        (0, 140, 70),    // 8  bright black
        (200, 255, 100), // 9  bright red
        (80, 255, 130),  // 10 bright green
        (230, 255, 110), // 11 bright yellow
        (60, 255, 200),  // 12 bright blue
        (170, 255, 180), // 13 bright magenta
        (100, 255, 230), // 14 bright cyan
        (210, 255, 220), // 15 bright white
    ],
    dark: true,
    grain: 1.2,
    crt: Some(CrtStyle {
        curvature: 0.0,
        scanline: 0.22,
        glow: 0.95,
        glow_radius: 7.0,
        corner: 0.0,
        flicker: 0.07,
    }),
    modern: None,
};

/// **Neon amber phosphor** (P3, Tron-grid): saturated amber traced over a
/// deep cool near-black tube — the phosphor still runs hot orange even
/// though, like every CRT preset, the tube glass itself reads cool black.
/// Style: the warmest raster — the deepest scanlines and the most nervous
/// flicker of the family, with a modest halo: an aging P3 workhorse whose
/// lines you can count.
pub static CRT_AMBER: Theme = Theme {
    page_bg: (6, 5, 6),
    ink: (255, 184, 0),
    text_muted: (226, 148, 0),
    term_fg: (255, 184, 0),
    term_bg: (6, 5, 6),
    // Unfocused borders sit back (focus-led hierarchy, as in paper-dark).
    border_normal: (112, 70, 16),
    border_focused: (255, 165, 20),
    border_thickness: 3.5,
    legend_off: (180, 115, 20),
    accent_default: (255, 200, 30),
    status_fg: (255, 200, 70),
    broadcast: (255, 170, 110),
    activity: (255, 160, 20),
    bell: (255, 190, 40),
    dim: (130, 85, 25),
    placeholder: (155, 100, 25),
    hint_fg: (172, 110, 25),
    find_hl_bg: (75, 48, 10),
    ansi: [
        (60, 35, 10),    // 0  black
        (255, 120, 40),  // 1  red
        (240, 200, 40),  // 2  green
        (255, 200, 30),  // 3  yellow
        (255, 160, 90),  // 4  blue
        (255, 140, 90),  // 5  magenta
        (250, 190, 110), // 6  cyan
        (255, 205, 120), // 7  white
        (150, 95, 35),   // 8  bright black
        (255, 140, 60),  // 9  bright red
        (255, 220, 60),  // 10 bright green
        (255, 215, 70),  // 11 bright yellow
        (255, 180, 110), // 12 bright blue
        (255, 160, 110), // 13 bright magenta
        (255, 210, 140), // 14 bright cyan
        (255, 225, 160), // 15 bright white
    ],
    dark: true,
    grain: 1.2,
    crt: Some(CrtStyle {
        curvature: 0.0,
        scanline: 0.26,
        glow: 0.85,
        glow_radius: 6.0,
        corner: 0.0,
        flicker: 0.08,
    }),
    modern: None,
};

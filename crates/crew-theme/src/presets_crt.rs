//! The CRT tubes: the green and amber phosphors. Each preset carries its
//! own `CrtStyle` — a coarse, jittery raster etched into the glass.
//!
//! Both run a 3.5px frame and sit on slabs of glass lit by their own
//! phosphor (`glass::tube_glass`). Since 2026-10-09 the window is
//! see-through like every mode's (the user: dark, light and CRT "with the
//! same pattern as glass"): the tube's faceplate is the dark mode's deeper
//! smoke over the desktop ([`crate::presets_glass_clear::DARK_LIQUID`]),
//! which phosphor green needs to hold 4.5:1 over a white desktop. The cool
//! pair, blue and violet, folded into them.

use crate::{CrtStyle, ModernStyle, Theme};

/// **Neon green phosphor** (P1, Tron-grid): hot saturated green traced over
/// a deep cool near-black tube, with a monochrome-green ANSI palette
/// (brightness tiers, faint hue tilts) for that single-gun terminal look.
/// Style: the hottest tube of the four — a raster etched into its glass, a
/// strong but tight bloom, and the jumpiest streaming flicker: a P1 tube
/// driven hard.
pub static CRT_GREEN: Theme = Theme {
    page_bg: (2, 6, 5),
    ink: (0, 242, 96),
    text_muted: (0, 194, 75),
    term_fg: (0, 255, 102),
    term_bg: (2, 6, 5),
    // Unfocused borders sit back (matching paper-dark's focus-led hierarchy)
    // so the bright phosphor frame alone says which pane is live.
    border_normal: (0, 90, 30),
    border_focused: (0, 255, 120),
    border_thickness: 3.5,
    legend_off: (0, 181, 70),
    accent_default: (30, 255, 140),
    status_fg: (190, 255, 80),
    broadcast: (150, 255, 150),
    activity: (0, 255, 110),
    bell: (200, 255, 90),
    dim: (0, 165, 63),
    placeholder: (0, 167, 64),
    hint_fg: (0, 172, 66),
    find_hl_bg: (10, 70, 30),
    ansi: [
        (93, 96, 95),    // 0  black
        (93, 170, 105),  // 1  red
        (106, 184, 118), // 2  green
        (120, 198, 131), // 3  yellow
        (134, 213, 145), // 4  blue
        (148, 227, 158), // 5  magenta
        (162, 242, 172), // 6  cyan
        (208, 212, 211), // 7  white
        (129, 132, 132), // 8  bright black
        (113, 190, 124), // 9  bright red
        (126, 204, 137), // 10 bright green
        (140, 219, 150), // 11 bright yellow
        (154, 234, 165), // 12 bright blue
        (168, 248, 178), // 13 bright magenta
        (206, 255, 211), // 14 bright cyan
        (230, 235, 233), // 15 bright white
    ],
    dark: true,
    crt: Some(CrtStyle {
        scanline: 0.0,
        glow: 0.95,
        glow_radius: 7.0,
        flicker: 0.07,
        core: 0.6,
        etch: 0.02,
        shade: CrtStyle::TUBE_SHADE,
    }),
    modern: Some(ModernStyle {
        pole_a: (106, 184, 118),
        pole_b: (162, 242, 172),
        drift_ms: 6_000,
    }),
    liquid: Some(crate::presets_glass_clear::DARK_LIQUID),
};

/// **Neon amber phosphor** (P3, Tron-grid): saturated amber traced over a
/// deep cool near-black tube — the phosphor still runs hot orange even
/// though, like every CRT preset, the tube glass itself reads cool black.
/// Style: the warmest tube — a fine raster etched into its glass and the
/// most nervous flicker of the family, with a modest halo: a P3 workhorse
/// whose lines you can still count.
pub static CRT_AMBER: Theme = Theme {
    page_bg: (6, 5, 6),
    ink: (254, 202, 103),
    text_muted: (210, 160, 56),
    term_fg: (255, 184, 0),
    term_bg: (6, 5, 6),
    // Unfocused borders sit back (focus-led hierarchy, as in paper-dark).
    border_normal: (101, 72, 0),
    border_focused: (255, 165, 20),
    border_thickness: 3.5,
    legend_off: (197, 148, 41),
    accent_default: (255, 200, 30),
    status_fg: (255, 200, 70),
    broadcast: (255, 170, 110),
    activity: (255, 160, 20),
    bell: (255, 190, 40),
    dim: (183, 134, 16),
    placeholder: (186, 136, 22),
    hint_fg: (189, 140, 28),
    find_hl_bg: (75, 48, 10),
    ansi: [
        (96, 95, 96),    // 0  black
        (170, 124, 17),  // 1  red
        (184, 138, 39),  // 2  green
        (198, 151, 56),  // 3  yellow
        (213, 165, 72),  // 4  blue
        (227, 179, 87),  // 5  magenta
        (242, 193, 102), // 6  cyan
        (212, 211, 211), // 7  white
        (132, 132, 132), // 8  bright black
        (190, 143, 47),  // 9  bright red
        (204, 158, 63),  // 10 bright green
        (219, 171, 78),  // 11 bright yellow
        (234, 185, 93),  // 12 bright blue
        (248, 199, 108), // 13 bright magenta
        (255, 216, 146), // 14 bright cyan
        (234, 233, 234), // 15 bright white
    ],
    dark: true,
    crt: Some(CrtStyle {
        scanline: 0.0,
        glow: 0.85,
        glow_radius: 6.0,
        flicker: 0.08,
        core: 0.6,
        etch: 0.012,
        shade: CrtStyle::TUBE_SHADE,
    }),
    modern: Some(ModernStyle {
        pole_a: (184, 138, 39),
        pole_b: (242, 193, 102),
        drift_ms: 6_000,
    }),
    liquid: Some(crate::presets_glass_clear::DARK_LIQUID),
};

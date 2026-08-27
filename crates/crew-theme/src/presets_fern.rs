//! **Fern**: a faint mint page under a deep green-teal light — the cool,
//! planted end of the light pool, where paper-light is neutral warm and
//! sepia-light is aged cream. The only light page whose accent is green, so
//! it can never be mistaken for either of them at a glance.
//!
//! Derived roles come from the ramp, the alarm and the wash — see
//! [`super::presets_harbor`].
use crate::{CrtStyle, ModernStyle, Theme};

pub static FERN: Theme = Theme {
    page_bg: (244, 248, 246),
    ink: (16, 28, 30),
    text_muted: (46, 56, 56),
    term_fg: (20, 33, 36),
    term_bg: (244, 248, 246),
    border_normal: (174, 178, 176),
    border_focused: (22, 156, 130),
    border_thickness: 2.5,
    legend_off: (90, 97, 96),
    accent_default: (10, 128, 104),
    status_fg: (23, 75, 131),
    broadcast: (152, 32, 152),
    activity: (28, 80, 199),
    bell: (183, 26, 20),
    dim: (107, 113, 112),
    placeholder: (116, 121, 119),
    hint_fg: (106, 112, 111),
    find_hl_bg: (153, 200, 180),
    ansi: [
        (34, 35, 35),   // 0  black
        (144, 67, 59),  // 1  red
        (34, 103, 52),  // 2  green
        (116, 85, 0),   // 3  yellow
        (41, 92, 149),  // 4  blue
        (123, 71, 129), // 5  magenta
        (0, 101, 104),  // 6  cyan
        (71, 72, 72),   // 7  white
        (97, 98, 98),   // 8  bright black
        (126, 50, 43),  // 9  bright red
        (11, 86, 36),   // 10  bright green
        (96, 70, 0),    // 11  bright yellow
        (23, 75, 131),  // 12  bright blue
        (106, 55, 112), // 13  bright magenta
        (0, 83, 86),    // 14  bright cyan
        (36, 37, 37),   // 15  bright white
    ],
    dark: false,
    grain: 1.2,
    crt: Some(CrtStyle {
        scanline: 0.0,
        glow: 0.35,
        glow_radius: 9.0,
        flicker: 0.010,
    }),
    modern: Some(ModernStyle {
        pole_a: (10, 128, 104),
        pole_b: (38, 91, 216),
        drift_ms: 6_000,
        dots: 0.16,
        wash: 0.12,
    }),
};

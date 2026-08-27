//! **crt-violet**: the fourth tube. Green, amber and blue are the phosphors
//! everyone remembers; violet is the one on the vector displays and the early
//! plasma panels, and it is the only tube whose glow does not read as either
//! "terminal" or "warning".
//!
//! Written like its siblings — a near-black page with one phosphor hue
//! climbing the whole ANSI ladder — and, like every palette, its derived roles
//! are what the ramp, the wash and the alarm produce rather than what looked
//! right by hand.
use crate::{CrtStyle, ModernStyle, Theme};

pub static CRT_VIOLET: Theme = Theme {
    page_bg: (6, 3, 10),
    ink: (223, 200, 249),
    text_muted: (181, 158, 205),
    term_fg: (218, 172, 255),
    term_bg: (6, 3, 10),
    border_normal: (89, 68, 110),
    border_focused: (198, 130, 255),
    border_thickness: 3.5,
    legend_off: (151, 129, 175),
    accent_default: (206, 148, 255),
    status_fg: (222, 170, 255),
    broadcast: (255, 160, 235),
    activity: (198, 140, 255),
    bell: (255, 170, 190),
    dim: (108, 86, 129),
    placeholder: (127, 104, 149),
    hint_fg: (140, 118, 163),
    find_hl_bg: (52, 32, 76),
    ansi: [
        (97, 94, 99),    // 0  black
        (149, 110, 189), // 1  red
        (163, 123, 203), // 2  green
        (177, 136, 218), // 3  yellow
        (191, 150, 232), // 4  blue
        (205, 164, 247), // 5  magenta
        (217, 181, 255), // 6  cyan
        (212, 210, 216), // 7  white
        (133, 130, 136), // 8  bright black
        (168, 129, 210), // 9  bright red
        (183, 142, 224), // 10  bright green
        (197, 156, 239), // 11  bright yellow
        (211, 170, 253), // 12  bright blue
        (221, 189, 255), // 13  bright magenta
        (231, 209, 255), // 14  bright cyan
        (234, 232, 238), // 15  bright white
    ],
    dark: true,
    grain: 1.2,
    crt: Some(CrtStyle {
        scanline: 0.08,
        glow: 1.0,
        glow_radius: 14.0,
        flicker: 0.035,
    }),
    modern: Some(ModernStyle {
        pole_a: (204, 150, 240),
        pole_b: (176, 150, 230),
        drift_ms: 6_000,
        dots: 0.10,
        wash: 0.10,
    }),
};

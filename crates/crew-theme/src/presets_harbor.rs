//! **Harbor**: a deep blue-slate page under an azure light — the cool end of
//! the dark pool, where paper-dark is neutral and sepia-dark is warm. Its
//! gradient runs azure into teal, so the ring reads as water rather than as
//! the aurora `nebula` wears.
//!
//! Every derived role (`text_muted` down to `border_normal`, plus `bell` and
//! `find_hl_bg`) is what the ramp, the alarm and the wash produce for this
//! page and ink — the parity tests in `ramp_tests`, `signal_tests` and
//! `highlight_tests` are what put the numbers here.
use crate::{CrtStyle, ModernStyle, Theme};

pub static HARBOR: Theme = Theme {
    page_bg: (13, 18, 25),
    ink: (234, 239, 242),
    text_muted: (194, 201, 206),
    term_fg: (233, 237, 242),
    term_bg: (13, 18, 25),
    border_normal: (65, 71, 79),
    border_focused: (23, 148, 211),
    border_thickness: 3.0,
    legend_off: (139, 145, 153),
    accent_default: (77, 172, 241),
    status_fg: (235, 181, 19),
    broadcast: (220, 118, 235),
    activity: (90, 161, 242),
    bell: (248, 129, 116),
    dim: (120, 127, 134),
    placeholder: (112, 118, 127),
    hint_fg: (121, 128, 136),
    find_hl_bg: (39, 64, 94),
    ansi: [
        (98, 101, 104),  // 0  black
        (255, 161, 149), // 1  red
        (122, 203, 137), // 2  green
        (223, 180, 85),  // 3  yellow
        (134, 189, 255), // 4  blue
        (228, 163, 236), // 5  magenta
        (46, 205, 211),  // 6  cyan
        (217, 220, 224), // 7  white
        (135, 138, 141), // 8  bright black
        (255, 193, 184), // 9  bright red
        (142, 224, 157), // 10  bright green
        (244, 200, 106), // 11  bright yellow
        (171, 209, 255), // 12  bright blue
        (247, 185, 255), // 13  bright magenta
        (77, 226, 232),  // 14  bright cyan
        (239, 243, 247), // 15  bright white
    ],
    dark: true,
    grain: 1.2,
    crt: Some(CrtStyle {
        scanline: 0.0,
        glow: 0.80,
        glow_radius: 11.0,
        flicker: 0.015,
    }),
    modern: Some(ModernStyle {
        pole_a: (59, 141, 233),
        pole_b: (36, 186, 202),
        drift_ms: 6_000,
        dots: 0.20,
        wash: 0.15,
    }),
};

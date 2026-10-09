//! **Glass**, the light pair: the iPhone's liquid glass by day (the user,
//! 2026-10-08: "I was expecting iphone liquid glass … I like light color
//! glass"). The window is see-through and paints no wallpaper of its own
//! (2026-10-09: "I don't think we need gradient colors in glass themes"):
//! the desktop is the background. Every pane is a slab of glass over it,
//! frosted WHITE in its body so dark ink reads on it, a hard white specular
//! edge riding on top and a soft grey shadow under it ([`crate::LiquidStyle`],
//! `glass::liquid_glass`).
//!
//! Two palettes, so `glass` rotates as every crew theme does: **sky** and
//! **dawn**, whose poles now only light the focus ring. Ink is the iPhone's
//! near-black label; the accents are its system blue and pink, deepened
//! until they read as text on a white slab.
//!
//! Every derived role is what the ramp, the alarm and the wash produce for
//! the page and ink — the parity tests in `ramp_tests`, `ansi_tests`,
//! `signal_tests` and `highlight_tests` put the numbers here.
use crate::{CrtStyle, LiquidStyle, ModernStyle, Theme};

/// The light glass's optics, shared by both wallpapers: a wider lens and a
/// deeper frost than the night glass — on a bright page the bend is what says
/// "glass", and the frost is what keeps black text calm on a vivid field.
const LIQUID_LIGHT: LiquidStyle = LiquidStyle {
    refract: 20.0,
    bevel: 22.0,
    blur: 22.0,
    dispersion: 0.08,
    clear_rim: 0.85,
    vibrance: 1.25,
    // The desktop shows three quarters through the gaps and nearly half
    // through a pane: as clear as the frost can be and still hold the text
    // over a black desktop (`liquid_tests::glass_text_reads_over_any_desktop`).
    window: 0.25,
    body: 0.40,
    desktop_blur: 20.0,
};

/// A whisper of bloom: a white page needs almost none before light reads as
/// haze, and the specular rim is what shines here, not a glow. Dawn's warm
/// light spreads a little further than the sky's (and no two palettes may
/// share a tuning — `the_phosphors_have_distinct_personalities`).
const fn bloom(glow: f32, glow_radius: f32) -> CrtStyle {
    CrtStyle {
        scanline: 0.0,
        glow,
        glow_radius,
        flicker: 0.0,
        core: 0.0,
        etch: 0.0,
        shade: 0.0,
    }
}

pub static GLASS_SKY: Theme = Theme {
    page_bg: (236, 242, 252),
    ink: (18, 20, 31),
    text_muted: (47, 51, 62),
    term_fg: (18, 20, 31),
    term_bg: (236, 242, 252),
    border_normal: (168, 173, 183),
    border_focused: (0, 102, 224),
    border_thickness: 2.5,
    legend_off: (88, 92, 102),
    accent_default: (0, 96, 214),
    status_fg: (23, 75, 131),
    broadcast: (152, 32, 152),
    activity: (28, 80, 199),
    bell: (183, 26, 20),
    dim: (104, 109, 119),
    placeholder: (112, 117, 127),
    hint_fg: (102, 108, 117),
    find_hl_bg: (150, 187, 249),
    ansi: [
        (30, 31, 34),   // 0  black
        (141, 63, 56),  // 1  red
        (29, 99, 49),   // 2  green
        (112, 82, 0),   // 3  yellow
        (38, 88, 145),  // 4  blue
        (119, 68, 126), // 5  magenta
        (0, 97, 101),   // 6  cyan
        (68, 69, 73),   // 7  white
        (92, 95, 98),   // 8  bright black
        (123, 46, 41),  // 9  bright red
        (4, 82, 33),    // 10 bright green
        (92, 67, 0),    // 11 bright yellow
        (20, 71, 127),  // 12 bright blue
        (102, 52, 109), // 13 bright magenta
        (0, 79, 83),    // 14 bright cyan
        (32, 33, 36),   // 15 bright white
    ],
    dark: false,
    grain: 0.0,
    crt: Some(bloom(0.25, 8.0)),
    modern: Some(ModernStyle {
        pole_a: (40, 104, 232),
        pole_b: (204, 52, 132),
        drift_ms: 6_000,
        dots: 0.0,
        // No wallpaper of its own: the desktop IS the background (the user,
        // 2026-10-09: "I don't think we need gradient colors in glass
        // themes"). The poles still light the focus ring.
        wash: 0.0,
    }),
    liquid: Some(LIQUID_LIGHT),
};

pub static GLASS_DAWN: Theme = Theme {
    page_bg: (252, 244, 240),
    ink: (33, 21, 27),
    text_muted: (63, 50, 52),
    term_fg: (33, 21, 27),
    term_bg: (252, 244, 240),
    border_normal: (183, 174, 171),
    border_focused: (204, 40, 110),
    border_thickness: 2.5,
    legend_off: (103, 92, 91),
    accent_default: (190, 30, 100),
    status_fg: (23, 75, 131),
    broadcast: (152, 32, 152),
    activity: (28, 80, 199),
    bell: (183, 26, 20),
    dim: (119, 109, 107),
    placeholder: (127, 117, 115),
    hint_fg: (117, 108, 105),
    find_hl_bg: (247, 165, 182),
    ansi: [
        (36, 33, 32),   // 0  black
        (143, 66, 58),  // 1  red
        (32, 102, 52),  // 2  green
        (115, 84, 0),   // 3  yellow
        (41, 91, 148),  // 4  blue
        (122, 70, 128), // 5  magenta
        (0, 100, 103),  // 6  cyan
        (73, 71, 70),   // 7  white
        (99, 96, 95),   // 8  bright black
        (125, 49, 43),  // 9  bright red
        (8, 85, 36),    // 10 bright green
        (95, 69, 0),    // 11 bright yellow
        (23, 74, 130),  // 12 bright blue
        (105, 54, 111), // 13 bright magenta
        (0, 82, 85),    // 14 bright cyan
        (38, 35, 34),   // 15 bright white
    ],
    dark: false,
    grain: 0.0,
    crt: Some(bloom(0.28, 9.0)),
    modern: Some(ModernStyle {
        pole_a: (196, 84, 40),
        pole_b: (124, 84, 226),
        drift_ms: 6_000,
        dots: 0.0,
        // No wallpaper of its own: the desktop IS the background (the user,
        // 2026-10-09: "I don't think we need gradient colors in glass
        // themes"). The poles still light the focus ring.
        wash: 0.0,
    }),
    liquid: Some(LIQUID_LIGHT),
};

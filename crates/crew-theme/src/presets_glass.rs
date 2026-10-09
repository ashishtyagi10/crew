//! **Glass night**: the iPhone's liquid glass after dark (the user,
//! 2026-10-07), the navy twin of `glass-clear` — the two are what `glass`
//! serves since the user asked for white words on glass (2026-10-09; see
//! [`crate::presets_glass_clear`]). Every pane is a slab of clear smoked
//! glass over the desktop — the window is see-through and paints no
//! wallpaper of its own — with a bright specular edge on top
//! ([`crate::LiquidStyle`], `glass::liquid_glass`). The accent is the
//! iPhone's dark-mode mint — aqua, the colour of the glass itself — and the
//! face is SF Mono.
//!
//! Every derived role is what the ramp, the alarm and the wash produce for
//! this page and ink — the parity tests in `ramp_tests`, `signal_tests` and
//! `highlight_tests` are what put the numbers here.
use crate::{CrtStyle, ModernStyle, Theme};

pub static GLASS_NIGHT: Theme = Theme {
    page_bg: (11, 9, 30),
    ink: (233, 234, 236),
    text_muted: (193, 196, 208),
    term_fg: (233, 234, 236),
    term_bg: (11, 9, 30),
    border_normal: (65, 66, 90),
    border_focused: (226, 232, 246),
    border_thickness: 2.0,
    legend_off: (138, 140, 160),
    accent_default: (99, 230, 226),
    status_fg: (255, 214, 10),
    broadcast: (191, 90, 242),
    activity: (100, 210, 255),
    bell: (255, 105, 97),
    dim: (120, 121, 144),
    placeholder: (112, 113, 136),
    hint_fg: (121, 123, 145),
    find_hl_bg: (44, 52, 112),
    ansi: [
        (97, 97, 108),   // 0  black
        (255, 155, 142), // 1  red
        (118, 200, 134), // 2  green
        (219, 176, 81),  // 3  yellow
        (127, 186, 255), // 4  blue
        (225, 159, 232), // 5  magenta
        (39, 201, 207),  // 6  cyan
        (214, 214, 227), // 7  white
        (133, 134, 145), // 8  bright black
        (255, 187, 177), // 9  bright red
        (138, 221, 153), // 10 bright green
        (240, 196, 102), // 11 bright yellow
        (165, 206, 255), // 12 bright blue
        (246, 179, 253), // 13 bright magenta
        (72, 222, 228),  // 14 bright cyan
        (236, 237, 249), // 15 bright white
    ],
    dark: true,
    // The wallpaper is glass and light, not newsprint.
    grain: 0.0,
    // The clear glass's bloom, glowing a little more on the deeper smoke.
    crt: Some(CrtStyle {
        glow: 0.55,
        ..crate::presets_glass_clear::CLEAR_BLOOM
    }),
    modern: Some(ModernStyle {
        pole_a: (70, 130, 255),
        pole_b: (255, 64, 150),
        drift_ms: 6_000,
        dots: 0.0,
        // No wallpaper of its own: the desktop IS the background (the user,
        // 2026-10-09: "I don't think we need gradient colors in glass
        // themes"). The poles still light the focus ring.
        wash: 0.0,
    }),
    // Clear glass, as `glass-clear` is: the shadow behind the words, not
    // thick smoke, is what holds white text over a white desktop.
    liquid: Some(crate::presets_glass_clear::CLEAR_LIQUID),
};

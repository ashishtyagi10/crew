//! **Glass night**: the iPhone's liquid glass after dark (the user,
//! 2026-10-07), the navy twin of `glass-clear` — the two are what `glass`
//! serves since the user asked for white words on glass (2026-10-09; see
//! [`crate::presets_glass_clear`]). Every pane is a slab of clear smoked
//! glass over the desktop — the window is see-through and paints no
//! wallpaper of its own — with a bright specular edge on top
//! ([`crate::LiquidStyle`], `glass::liquid_glass`). The accent is the
//! iPhone's dark-mode mint — aqua, the colour of the glass itself — the
//! focus ring an aurora from that aqua green to violet, and the face is SF
//! Mono.
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
        (254, 152, 171), // 1  red
        (116, 200, 135), // 2  green
        (219, 176, 81),  // 3  yellow
        (119, 187, 255), // 4  blue
        (204, 165, 249), // 5  magenta
        (45, 202, 194),  // 6  cyan
        (214, 214, 227), // 7  white
        (133, 134, 145), // 8  bright black
        (255, 185, 196), // 9  bright red
        (136, 221, 154), // 10 bright green
        (240, 196, 102), // 11 bright yellow
        (159, 206, 255), // 12 bright blue
        (220, 191, 255), // 13 bright magenta
        (76, 223, 214),  // 14 bright cyan
        (236, 237, 249), // 15 bright white
    ],
    dark: true,
    // The clear glass's bloom, glowing a little more on the deeper smoke.
    crt: Some(CrtStyle {
        glow: 0.55,
        ..crate::presets_glass_clear::CLEAR_BLOOM
    }),
    // An aurora over the night sky: the mint's aqua green turning to violet.
    // Until 2026-10-29 these were glass-clear's blue→pink, bit for bit, so
    // the two glasses drew one focus ring (`ring_tests`).
    modern: Some(ModernStyle {
        pole_a: (20, 165, 140),
        pole_b: (190, 100, 230),
        drift_ms: 6_000,
    }),
    // Clear glass, as `glass-clear` is: the shadow behind the words, not
    // thick smoke, is what holds white text over a white desktop.
    liquid: Some(crate::presets_glass_clear::CLEAR_LIQUID),
    syntax: crate::syntaxslot::GLASS_NIGHT,
    selection: None,
};

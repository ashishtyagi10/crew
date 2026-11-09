//! PRISM: one palette, `prism` (2026-11-08, the user: a new 3D theme). It is
//! its look's whole pool, the way neon and purple are, so `/theme prism`
//! serves it and the `Ctrl+Shift+L` cycle and `auto` reach it.
//!
//! What makes it a 3D theme is not in this struct: [`crate::ThemeId::scene`]
//! names the scene crew-render draws behind its panes (drifting glass prisms
//! in a jade fog), on unless the user turned the backdrop off. The palette is
//! built for that scene: a jade page, a hue no other page holds, which the
//! scene's fog fades into; sheerer slabs than the smoked glass so the prisms
//! read through the panes; and a window dense enough that the scene, not a
//! busy wallpaper, is what lies behind the words.
//!
//! The text ladder, the ANSI slots and the search wash are what
//! [`crate::ramp`], [`crate::ansi`] and [`crate::highlight`] derive.

use crate::presets_glass_clear::CLEAR_LIQUID;
use crate::{CrtStyle, LiquidStyle, ModernStyle, Theme};

/// Prism's glass: a dark room like neon's, whose window keeps three
/// quarters of the desktop out (the scene is what should show, on the
/// page's own jade, not whatever wallpaper is behind), and each slab is as sheer as the clear glass's, sheerer than the smoked
/// [`crate::presets_glass_clear::DARK_LIQUID`], so the prisms stay visible
/// through a pane rather than only in the gutters.
pub(crate) const PRISM_LIQUID: LiquidStyle = LiquidStyle {
    window: 0.75,
    body: 0.45,
    ..CLEAR_LIQUID
};

/// **Prism**: a deep jade page, white words with an ice cast, a pale
/// ice-mint accent and frame, coral activity (the accent's complement, which
/// is also the scene's rim light), and the ring running teal to violet, the
/// two ends of a split beam.
pub static PRISM: Theme = Theme {
    page_bg: (4, 30, 26),
    ink: (231, 246, 241),
    text_muted: (192, 212, 206),
    term_fg: (231, 246, 241),
    term_bg: (4, 30, 26),
    border_normal: (55, 81, 75),
    border_focused: (120, 240, 205),
    border_thickness: 3.5,
    legend_off: (132, 156, 150),
    accent_default: (150, 255, 220),
    status_fg: (180, 240, 255),
    broadcast: (255, 140, 210),
    activity: (255, 176, 140),
    bell: (255, 215, 130),
    dim: (112, 137, 131),
    placeholder: (103, 128, 122),
    hint_fg: (113, 138, 132),
    find_hl_bg: (20, 90, 80),
    ansi: [
        (97, 108, 106),  // 0  black
        (255, 171, 159), // 1  red
        (112, 212, 157), // 2  green
        (213, 193, 92),  // 3  yellow
        (117, 202, 255), // 4  blue
        (206, 180, 255), // 5  magenta
        (75, 214, 191),  // 6  cyan
        (218, 230, 227), // 7  white
        (134, 146, 143), // 8  bright black
        (255, 202, 194), // 9  bright red
        (133, 233, 177), // 10 bright green
        (234, 214, 113), // 11 bright yellow
        (166, 220, 255), // 12 bright blue
        (222, 207, 255), // 13 bright magenta
        (100, 235, 211), // 14 bright cyan
        (241, 254, 251), // 15 bright white
    ],
    dark: true,
    crt: Some(CrtStyle {
        scanline: 0.0,
        glow: 0.5,
        glow_radius: 16.0,
        flicker: 0.0,
        core: 0.0,
        etch: 0.0,
        shade: CrtStyle::GLASS_SHADE,
    }),
    modern: Some(ModernStyle {
        pole_a: (20, 184, 166),
        pole_b: (139, 92, 246),
        drift_ms: 6_000,
    }),
    liquid: Some(PRISM_LIQUID),
    syntax: crate::syntaxslot::PRISM,
    selection: None,
};

#[cfg(test)]
#[path = "prism_tests.rs"]
mod tests;

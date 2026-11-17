//! METAL: one palette, `mercury` (2026-11-17, the user: "add a new liquid
//! metal theme for crew"). It is its look's whole pool, the way neon, purple
//! and prism are, so `/theme metal` serves it and the `Ctrl+Shift+L` cycle
//! and `auto` reach it.
//!
//! Liquid metal is mostly not in this struct, as prism's prisms are not:
//! [`crate::ThemeId::scene`] names the scene crew-render draws behind the
//! panes (droplets of mercury wobbling in a gunmetal fog, lit cold from
//! the key and warm from the rim), on unless the user turned the backdrop off.
//!
//! The palette is the metal: a gunmetal page, a STEEL accent. Silver is what
//! one reaches for, but a near-white silver on a near-black page is the
//! Ink Reader's own white-on-black (Δ 0.039 from paper-dark), and an icy
//! chrome blue sits on tokyo-night (Δ 0.042). A mid-grey steel, darker than
//! every other dark accent and almost without hue, is what clears
//! `no_two_palettes_are_near_duplicates` (nearest glass-night, Δ ≈ 0.055,
//! then tokyo-night, Δ ≈ 0.061).
//!
//! The text ladder, the ANSI slots and the search wash are what
//! [`crate::ramp`], [`crate::ansi`] and [`crate::highlight`] derive.

use crate::presets_prism::PRISM_LIQUID;
use crate::{CrtStyle, LiquidStyle, ModernStyle, Theme};

/// Mercury's glass: prism's dark room (the scene, not the wallpaper, is
/// what lies behind the words), its slabs a touch sheerer still, so the
/// chrome behind a pane still shines rather than reading as dark oil.
pub(crate) const METAL_LIQUID: LiquidStyle = LiquidStyle {
    body: 0.34,
    ..PRISM_LIQUID
};

/// **Mercury**: a gunmetal page, white words with a cold cast, a steel
/// accent and frame, a warm copper for activity (the scene's rim light, the
/// warm half of polished metal's two-tone shine), and the ring running
/// silver to blued steel.
pub static MERCURY: Theme = Theme {
    page_bg: (12, 13, 16),
    ink: (232, 235, 238),
    text_muted: (194, 197, 201),
    term_fg: (232, 235, 238),
    term_bg: (12, 13, 16),
    border_normal: (67, 69, 72),
    border_focused: (200, 212, 228),
    border_thickness: 3.5,
    legend_off: (140, 142, 146),
    accent_default: (175, 186, 202),
    status_fg: (180, 230, 255),
    broadcast: (255, 140, 210),
    activity: (255, 186, 140),
    bell: (212, 202, 104),
    dim: (122, 123, 128),
    placeholder: (114, 115, 119),
    hint_fg: (123, 125, 129),
    find_hl_bg: (60, 72, 92),
    ansi: [
        (98, 99, 100),   // 0  black
        (255, 156, 143), // 1  red
        (118, 200, 134), // 2  green
        (220, 176, 82),  // 3  yellow
        (128, 186, 255), // 4  blue
        (225, 160, 232), // 5  magenta
        (41, 202, 208),  // 6  cyan
        (215, 216, 217), // 7  white
        (134, 135, 137), // 8  bright black
        (255, 188, 178), // 9  bright red
        (138, 221, 153), // 10 bright green
        (241, 196, 103), // 11 bright yellow
        (165, 206, 255), // 12 bright blue
        (246, 180, 253), // 13 bright magenta
        (73, 223, 229),  // 14 bright cyan
        (238, 239, 240), // 15 bright white
    ],
    dark: true,
    crt: Some(CrtStyle {
        scanline: 0.0,
        glow: 0.65,
        glow_radius: 13.0,
        flicker: 0.0,
        core: 0.0,
        etch: 0.0,
        shade: CrtStyle::GLASS_SHADE,
    }),
    modern: Some(ModernStyle {
        pole_a: (214, 222, 235),
        pole_b: (92, 124, 178),
        drift_ms: 6_000,
    }),
    liquid: Some(METAL_LIQUID),
    syntax: crate::syntaxslot::MERCURY,
    selection: None,
};

#[cfg(test)]
#[path = "metal_tests.rs"]
mod tests;

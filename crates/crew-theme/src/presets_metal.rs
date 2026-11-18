//! METAL: two palettes of brushed metal plates (the user, 2026-11-17: "add
//! a new liquid metal theme for crew", then "with metal theme I meant actual
//! metal look, instead of black and just some baloon in it .. some shiny
//! look"). The first metal look was gunmetal glass over a 3D scene of
//! mercury blobs; on a near-black page the only metal was the blobs. Now
//! the panes ARE the metal and there is no scene.
//!
//! A metal palette is a relief one ([`LiquidStyle::relief`]): no frame is
//! stroked, every card is a solid slab of the page's colour standing out of
//! it. [`LiquidStyle::metal`] makes each slab a brushed plate: hairline
//! grain along x, a sheen streak across the grain that the pointer slides,
//! and a machined chamfer, lit toward the light and dark away from it. A
//! dark plate rests a step above its page ([`crate::METAL_REST_DARK`]); the
//! face swings at most [`crate::METAL_SWING`] from there, and `metal_tests`
//! reads every ink here on [`crate::metal_face`], the face as drawn.
//!
//! - `mercury`: dark gunmetal plates, white words, a rose-gold accent.
//! - `silver`: polished aluminium plates, engraved near-black words, a
//!   anodized-titanium violet accent.

use crate::presets_relief::RELIEF_LIQUID;
use crate::{CrtStyle, LiquidStyle, ModernStyle, Theme};

/// Metal's material: relief's solid, unframed slabs, brushed.
pub(crate) const METAL_LIQUID: LiquidStyle = LiquidStyle {
    metal: 1.0,
    ..RELIEF_LIQUID
};

/// Mercury's bloom: quieter than slate's, and no text shadow, since a
/// solid plate has no desktop behind its words.
const DARK_BLOOM: CrtStyle = CrtStyle {
    scanline: 0.0,
    glow: 0.40,
    glow_radius: 9.0,
    flicker: 0.0,
    core: 0.0,
    etch: 0.0,
    shade: 0.0,
};

/// **Mercury**: gunmetal plates, white words with a cold cast, a rose-gold
/// accent (the warm metal beside the cold one), a cool mint for activity,
/// and the ring running silver to rose gold. A steel-grey accent was tried
/// first: a grey reads as an Ink Reader page's hue-less accent everywhere a
/// hue is looked for, and every steel with a hue sat on tokyo-night. The page is a mid-dark steel, not
/// black: a brushed face needs light to show its grain and sheen.
pub static MERCURY: Theme = Theme {
    page_bg: (28, 29, 31),
    ink: (240, 242, 244),
    text_muted: (208, 211, 214),
    term_fg: (246, 248, 250),
    term_bg: (28, 29, 31),
    border_normal: (76, 78, 80),
    border_focused: (214, 164, 156),
    border_thickness: 2.0,
    legend_off: (151, 153, 155),
    accent_default: (214, 164, 156),
    status_fg: (178, 232, 255),
    broadcast: (240, 150, 210),
    activity: (140, 230, 200),
    bell: (236, 222, 120),
    dim: (132, 134, 136),
    placeholder: (124, 126, 128),
    hint_fg: (134, 136, 138),
    find_hl_bg: (111, 50, 40),
    ansi: [
        (107, 108, 108), // 0  black
        (255, 176, 165), // 1  red
        (132, 214, 147), // 2  green
        (233, 190, 95),  // 3  yellow
        (153, 199, 255), // 4  blue
        (239, 173, 246), // 5  magenta
        (62, 215, 221),  // 6  cyan
        (229, 231, 233), // 7  white
        (145, 146, 148), // 8  bright black
        (255, 206, 199), // 9  bright red
        (152, 235, 167), // 10 bright green
        (254, 211, 116), // 11 bright yellow
        (189, 219, 255), // 12 bright blue
        (250, 201, 255), // 13 bright magenta
        (89, 236, 242),  // 14 bright cyan
        (254, 255, 255), // 15 bright white
    ],
    dark: true,
    crt: Some(DARK_BLOOM),
    modern: Some(ModernStyle {
        pole_a: (214, 220, 230),
        pole_b: (214, 164, 156),
        drift_ms: 6_000,
    }),
    liquid: Some(METAL_LIQUID),
    syntax: crate::syntaxslot::MERCURY,
    selection: None,
};

/// **Silver**: polished aluminium plates, near-black words as if engraved,
/// an anodized-titanium violet accent. Colour is printed here as on the other light relief
/// pages: deep inks, nothing lit, no ring and no bloom.
pub static SILVER: Theme = Theme {
    page_bg: (224, 228, 234),
    ink: (2, 2, 4),
    text_muted: (41, 42, 48),
    term_fg: (12, 14, 18),
    term_bg: (224, 228, 234),
    border_normal: (159, 162, 168),
    border_focused: (72, 44, 128),
    border_thickness: 2.0,
    legend_off: (82, 84, 89),
    accent_default: (72, 44, 128),
    status_fg: (88, 60, 8),
    broadcast: (110, 26, 96),
    activity: (24, 100, 44),
    bell: (130, 30, 26),
    dim: (98, 100, 105),
    placeholder: (105, 108, 114),
    hint_fg: (97, 99, 104),
    find_hl_bg: (185, 161, 240),
    ansi: [
        (17, 18, 20),
        (132, 55, 48),
        (18, 91, 41),
        (102, 74, 0),
        (30, 80, 136),
        (110, 60, 116),
        (0, 88, 92),
        (60, 61, 63),
        (85, 86, 89),
        (114, 38, 33),
        (0, 74, 28),
        (83, 59, 0),
        (10, 64, 118),
        (93, 44, 99),
        (0, 71, 74),
        (20, 21, 23),
    ],
    dark: false,
    crt: None,
    modern: None,
    liquid: Some(METAL_LIQUID),
    syntax: crate::syntaxslot::SILVER,
    selection: None,
};

#[cfg(test)]
#[path = "metal_tests.rs"]
mod tests;

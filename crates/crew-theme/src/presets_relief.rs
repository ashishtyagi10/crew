//! RELIEF: neumorphism, four palettes (the user, 2026-11-12: "few more
//! themes with lots of 3D effect, like panel card, input box and no border
//! needed on pane, just an elevated card", then a neumorphism board: pale
//! grey cards the colour of their page, each shaped by a white shadow above
//! and a grey one below).
//!
//! No frame is stroked on these. Every card (a pane, the nav's panels) is a
//! solid slab of the page's own colour, raised out of it by two soft
//! shadows: a light one toward the light, upper left, and a dark one away
//! from it. The input bar is pressed INTO the page the same way round. Focus
//! is height and light: the card with the keys stands higher, and its light
//! shadow takes the accent. What draws it is [`crate::glass::relief_glass`]
//! and crew-render's glass pass, switched by [`LiquidStyle::relief`].
//!
//! Three of the four are LIGHT, which is where the look lives: a pale page
//! has a lighter shadow to throw (white) and a darker one, and the dark
//! roster is nearly full besides (`no_two_palettes_are_near_duplicates`).
//!
//! - `porcelain`: pale grey, dark ink, an indigo accent: the board itself.
//! - `sand`: warm beige, a terracotta accent.
//! - `mist`: cool blue-grey, a deep teal accent.
//! - `slate`: the dark one, blue-black, white words, a gold accent.
use crate::presets_glass_clear::CLEAR_LIQUID;
use crate::{CrtStyle, LiquidStyle, ModernStyle, Theme};

/// Relief's material: a window that keeps nearly all of the desktop out
/// (the shadows need the page to fall on) and slabs that are solid.
pub(crate) const RELIEF_LIQUID: LiquidStyle = LiquidStyle {
    window: 0.94,
    body: 1.0,
    relief: 1.0,
    ..CLEAR_LIQUID
};

/// Slate's bloom: the dark pool's quietest glow and no text shadow, since
/// a solid card has no desktop behind its words.
const DARK_BLOOM: CrtStyle = CrtStyle {
    scanline: 0.0,
    glow: 0.42,
    glow_radius: 10.0,
    flicker: 0.0,
    core: 0.0,
    etch: 0.0,
    shade: 0.0,
};

/// **Slate**: blue-grey slabs, white words with a cool cast, a gold accent
/// (the one warm thing on a cold page), and the ring's poles running gold
/// to periwinkle.
pub static SLATE: Theme = Theme {
    page_bg: (23, 26, 33),
    ink: (238, 242, 248),
    text_muted: (203, 208, 215),
    term_fg: (238, 242, 248),
    term_bg: (23, 26, 33),
    border_normal: (72, 76, 84),
    border_focused: (255, 214, 120),
    border_thickness: 2.0,
    legend_off: (146, 151, 159),
    accent_default: (255, 214, 120),
    status_fg: (150, 225, 255),
    broadcast: (240, 150, 235),
    activity: (140, 235, 190),
    bell: (255, 160, 130),
    dim: (127, 132, 140),
    placeholder: (119, 123, 131),
    hint_fg: (129, 133, 141),
    find_hl_bg: (96, 72, 10),
    ansi: [
        (104, 106, 109),
        (255, 171, 160),
        (129, 211, 145),
        (230, 187, 92),
        (147, 196, 255),
        (236, 170, 243),
        (58, 212, 218),
        (226, 228, 232),
        (142, 144, 147),
        (255, 202, 194),
        (149, 232, 165),
        (251, 207, 113),
        (183, 216, 255),
        (250, 196, 255),
        (86, 233, 239),
        (249, 251, 255),
    ],
    dark: true,
    crt: Some(DARK_BLOOM),
    modern: Some(ModernStyle {
        pole_a: (250, 204, 21),
        pole_b: (129, 140, 248),
        drift_ms: 6_000,
    }),
    liquid: Some(RELIEF_LIQUID),
    syntax: crate::SyntaxSlots::HOUSE,
    selection: None,
};

/// **Porcelain**: pale grey slabs on a pale grey page, near-black words and
/// an indigo accent. Colour is printed here as it is on the Ink Reader's
/// light page: deep inks, nothing lit, no ring and no bloom.
pub static PORCELAIN: Theme = Theme {
    page_bg: (234, 234, 236),
    ink: (13, 12, 14),
    text_muted: (46, 46, 49),
    term_fg: (20, 23, 30),
    term_bg: (234, 234, 236),
    border_normal: (167, 167, 168),
    border_focused: (67, 72, 190),
    border_thickness: 2.0,
    legend_off: (88, 88, 90),
    accent_default: (67, 72, 190),
    status_fg: (107, 78, 26),
    broadcast: (126, 40, 110),
    activity: (20, 96, 110),
    bell: (150, 50, 44),
    dim: (104, 104, 106),
    placeholder: (112, 112, 114),
    hint_fg: (103, 103, 104),
    find_hl_bg: (160, 160, 245),
    ansi: [
        (25, 25, 26),
        (136, 59, 52),
        (24, 95, 45),
        (107, 78, 0),
        (34, 84, 141),
        (114, 64, 120),
        (0, 92, 96),
        (64, 64, 65),
        (90, 90, 91),
        (118, 42, 37),
        (0, 78, 30),
        (88, 63, 0),
        (15, 68, 123),
        (97, 48, 103),
        (0, 75, 78),
        (27, 27, 28),
    ],
    dark: false,
    crt: None,
    modern: None,
    liquid: Some(RELIEF_LIQUID),
    syntax: crate::syntaxslot::PORCELAIN,
    selection: None,
};

/// **Sand**: warm beige slabs, near-black words and a terracotta accent:
/// porcelain by lamplight.
pub static SAND: Theme = Theme {
    page_bg: (238, 232, 222),
    ink: (11, 10, 12),
    text_muted: (49, 44, 47),
    term_fg: (20, 23, 30),
    term_bg: (238, 232, 222),
    border_normal: (172, 165, 156),
    border_focused: (168, 66, 32),
    border_thickness: 2.0,
    legend_off: (94, 85, 84),
    accent_default: (168, 66, 32),
    status_fg: (96, 82, 10),
    broadcast: (120, 44, 120),
    activity: (22, 92, 116),
    bell: (150, 24, 86),
    dim: (110, 102, 98),
    placeholder: (118, 109, 105),
    hint_fg: (109, 100, 97),
    find_hl_bg: (245, 161, 99),
    ansi: [
        (25, 23, 21),
        (135, 58, 51),
        (22, 94, 44),
        (106, 77, 0),
        (33, 83, 139),
        (113, 63, 119),
        (0, 91, 95),
        (65, 64, 60),
        (92, 89, 86),
        (117, 41, 36),
        (0, 77, 30),
        (87, 62, 0),
        (14, 67, 121),
        (96, 47, 102),
        (0, 74, 77),
        (27, 25, 23),
    ],
    dark: false,
    crt: None,
    modern: None,
    liquid: Some(RELIEF_LIQUID),
    syntax: crate::syntaxslot::SAND,
    selection: None,
};

/// **Mist**: cool blue-grey slabs, near-black words and a deep teal accent:
/// porcelain under an overcast sky.
pub static MIST: Theme = Theme {
    page_bg: (226, 233, 240),
    ink: (10, 9, 11),
    text_muted: (44, 45, 49),
    term_fg: (20, 23, 30),
    term_bg: (226, 233, 240),
    border_normal: (161, 166, 173),
    border_focused: (0, 104, 116),
    border_thickness: 2.0,
    legend_off: (84, 87, 93),
    accent_default: (0, 104, 116),
    status_fg: (104, 76, 20),
    broadcast: (124, 44, 112),
    activity: (48, 78, 150),
    bell: (152, 48, 44),
    dim: (100, 103, 109),
    placeholder: (107, 111, 117),
    hint_fg: (99, 102, 108),
    find_hl_bg: (79, 199, 212),
    ansi: [
        (22, 23, 25),
        (134, 58, 51),
        (22, 94, 44),
        (106, 77, 0),
        (33, 83, 139),
        (113, 62, 119),
        (0, 91, 95),
        (62, 64, 66),
        (87, 89, 92),
        (116, 41, 36),
        (0, 77, 30),
        (87, 62, 0),
        (14, 67, 121),
        (96, 46, 102),
        (0, 74, 77),
        (24, 25, 27),
    ],
    dark: false,
    crt: None,
    modern: None,
    liquid: Some(RELIEF_LIQUID),
    syntax: crate::syntaxslot::MIST,
    selection: None,
};

#[cfg(test)]
#[path = "relief_tests.rs"]
mod tests;

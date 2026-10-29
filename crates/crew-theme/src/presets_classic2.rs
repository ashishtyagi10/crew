//! CLASSIC, the warm three: Everforest, Gruvbox and Monokai. The family,
//! and what each palette keeps of its original, is in `presets_classic.rs`.

use crate::presets_classic::CLASSIC_BLOOM;
use crate::presets_glass_clear::DARK_LIQUID;
use crate::{CrtStyle, ModernStyle, Theme};

/// **Everforest** (dark): a forest at night, the green-grey `bg0` deepened,
/// warm parchment words, green for the frame, aqua for the accent and the
/// ring running green to aqua.
pub static EVERFOREST: Theme = Theme {
    page_bg: (21, 26, 30),
    ink: (254, 241, 212),
    text_muted: (197, 211, 191),
    term_fg: (254, 241, 212),
    term_bg: (21, 26, 30),
    border_normal: (68, 76, 81),
    border_focused: (167, 192, 128),
    border_thickness: 3.5,
    legend_off: (136, 154, 152),
    accent_default: (131, 192, 146),
    status_fg: (179, 215, 210),
    broadcast: (214, 153, 182),
    activity: (223, 180, 89),
    bell: (235, 154, 155),
    dim: (118, 134, 135),
    placeholder: (111, 125, 127),
    hint_fg: (120, 136, 137),
    find_hl_bg: (60, 84, 71),
    ansi: [
        (103, 106, 108), // 0  black
        (255, 170, 167), // 1  red
        (170, 204, 113), // 2  green
        (229, 186, 91),  // 3  yellow
        (66, 213, 198),  // 4  blue
        (255, 164, 212), // 5  magenta
        (124, 211, 147), // 6  cyan
        (224, 227, 230), // 7  white
        (141, 143, 146), // 8  bright black
        (255, 201, 198), // 9  bright red
        (190, 225, 133), // 10 bright green
        (250, 206, 112), // 11 bright yellow
        (92, 234, 218),  // 12 bright blue
        (255, 198, 227), // 13 bright magenta
        (144, 232, 167), // 14 bright cyan
        (248, 251, 253), // 15 bright white
    ],
    dark: true,
    crt: Some(CrtStyle {
        glow: 0.65,
        ..CLASSIC_BLOOM
    }),
    modern: Some(ModernStyle {
        pole_a: (167, 192, 128),
        pole_b: (131, 192, 146),
        drift_ms: 6_000,
    }),
    liquid: Some(DARK_LIQUID),
};

/// **Gruvbox** (dark): the retro groove's charcoal deepened a step (on
/// `#282828` the house ladder ran out of room and muted text came out as
/// bright as the ink), cream words, yellow for the frame, orange for the
/// accent and the ring running yellow to orange.
pub static GRUVBOX: Theme = Theme {
    page_bg: (25, 25, 25),
    ink: (255, 241, 205),
    text_muted: (224, 203, 186),
    term_fg: (255, 241, 205),
    term_bg: (25, 25, 25),
    border_normal: (77, 74, 74),
    border_focused: (250, 189, 47),
    border_thickness: 3.5,
    legend_off: (161, 147, 144),
    accent_default: (254, 147, 60),
    status_fg: (186, 216, 174),
    broadcast: (211, 134, 155),
    activity: (188, 191, 50),
    bell: (253, 145, 132),
    dim: (139, 128, 128),
    placeholder: (130, 120, 120),
    hint_fg: (141, 130, 129),
    find_hl_bg: (105, 68, 37),
    ansi: [
        (105, 105, 105), // 0  black
        (255, 171, 148), // 1  red
        (194, 198, 98),  // 2  green
        (231, 185, 91),  // 3  yellow
        (90, 212, 175),  // 4  blue
        (255, 166, 198), // 5  magenta
        (148, 207, 127), // 6  cyan
        (227, 227, 227), // 7  white
        (143, 143, 143), // 8  bright black
        (255, 201, 187), // 9  bright red
        (214, 219, 118), // 10 bright green
        (252, 205, 112), // 11 bright yellow
        (113, 233, 195), // 12 bright blue
        (255, 199, 218), // 13 bright magenta
        (168, 228, 147), // 14 bright cyan
        (250, 250, 250), // 15 bright white
    ],
    dark: true,
    crt: Some(CrtStyle {
        glow: 0.7,
        ..CLASSIC_BLOOM
    }),
    modern: Some(ModernStyle {
        pole_a: (250, 189, 47),
        pole_b: (254, 128, 25),
        drift_ms: 6_000,
    }),
    liquid: Some(DARK_LIQUID),
};

/// **Monokai**: the olive-black of the classic, deepened a step, white
/// words, its green for the accent, its pink for the frame and the ring
/// running pink to purple.
pub static MONOKAI: Theme = Theme {
    page_bg: (25, 26, 21),
    ink: (242, 242, 236),
    text_muted: (207, 208, 201),
    term_fg: (242, 242, 236),
    term_bg: (25, 26, 21),
    border_normal: (74, 76, 70),
    border_focused: (249, 38, 114),
    border_thickness: 3.5,
    legend_off: (150, 151, 144),
    accent_default: (166, 226, 46),
    status_fg: (123, 222, 241),
    broadcast: (174, 129, 255),
    activity: (219, 208, 100),
    bell: (253, 151, 31),
    dim: (131, 132, 125),
    placeholder: (122, 124, 117),
    hint_fg: (132, 133, 127),
    find_hl_bg: (30, 78, 4),
    ansi: [
        (105, 106, 103), // 0  black
        (255, 168, 184), // 1  red
        (167, 204, 115), // 2  green
        (205, 195, 94),  // 3  yellow
        (64, 209, 234),  // 4  blue
        (203, 181, 255), // 5  magenta
        (67, 213, 197),  // 6  cyan
        (227, 227, 224), // 7  white
        (143, 143, 141), // 8  bright black
        (255, 200, 209), // 9  bright red
        (187, 225, 135), // 10 bright green
        (225, 216, 115), // 11 bright yellow
        (91, 230, 255),  // 12 bright blue
        (220, 207, 255), // 13 bright magenta
        (93, 234, 217),  // 14 bright cyan
        (250, 251, 248), // 15 bright white
    ],
    dark: true,
    crt: Some(CrtStyle {
        glow: 0.8,
        ..CLASSIC_BLOOM
    }),
    modern: Some(ModernStyle {
        pole_a: (249, 38, 114),
        pole_b: (174, 129, 255),
        drift_ms: 6_000,
    }),
    liquid: Some(DARK_LIQUID),
};

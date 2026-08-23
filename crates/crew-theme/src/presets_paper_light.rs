//! Light paper-family presets: four newspaper pages (see presets_paper.rs for the family conventions).

use crate::Theme;

/// **Sepia light**: warm aged-newsprint cream page with deep brown-black
/// ink — the light twin of SEPIA_DARK, echoing its warm gold accent
/// character.
pub static SEPIA_LIGHT: Theme = Theme {
    page_bg: (245, 235, 205),
    ink: (19, 13, 8),
    text_muted: (55, 45, 34),
    term_fg: (20, 13, 8),
    term_bg: (245, 235, 205),
    border_normal: (178, 167, 141),
    border_focused: (120, 90, 50),
    border_thickness: 3.0,
    legend_off: (98, 87, 68),
    accent_default: (150, 90, 20),
    status_fg: (140, 90, 10),
    broadcast: (140, 50, 90),
    activity: (50, 80, 120),
    bell: (146, 93, 15),
    dim: (115, 104, 83),
    placeholder: (122, 112, 90),
    hint_fg: (113, 102, 81),
    find_hl_bg: (217, 178, 59),
    ansi: [
        (29, 26, 17),   // 0  black
        (136, 60, 52),  // 1  red
        (25, 96, 46),   // 2  green
        (108, 79, 0),   // 3  yellow
        (35, 85, 141),  // 4  blue
        (115, 64, 121), // 5  magenta
        (0, 93, 97),    // 6  cyan
        (69, 65, 56),   // 7  white
        (94, 91, 81),   // 8  bright black
        (118, 43, 37),  // 9  bright red
        (0, 79, 31),    // 10 bright green
        (89, 64, 0),    // 11 bright yellow
        (17, 69, 123),  // 12 bright blue
        (98, 48, 104),  // 13 bright magenta
        (0, 75, 79),    // 14 bright cyan
        (31, 28, 19),   // 15 bright white
    ],
    dark: false,
    grain: 1.2,
    crt: None,
    modern: None,
};

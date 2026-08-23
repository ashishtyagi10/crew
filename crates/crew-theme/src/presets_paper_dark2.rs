//! Dark paper-family presets, the second leaf — `presets_paper.rs` sits at
//! the line cap, so new dark pages land here. Same family conventions:
//! focus-led border hierarchy, muted-but-readable ANSI slots, 1.2 grain.

use crate::Theme;

/// **Moss blotter**: a deep desaturated moss-green desk blotter with warm
/// paper-white ink — the study-lamp page of the dark family; accents lean
/// botanical (fern, spruce, lichen).
pub static MOSS_BLOTTER: Theme = Theme {
    page_bg: (17, 21, 14),
    ink: (243, 241, 227),
    text_muted: (201, 203, 190),
    term_fg: (240, 238, 225),
    term_bg: (17, 21, 14),
    // Focus-led border hierarchy, as in paper-dark.
    border_normal: (69, 73, 64),
    border_focused: (214, 218, 190),
    border_thickness: 2.5,
    legend_off: (144, 147, 136),
    accent_default: (165, 205, 140),
    status_fg: (230, 198, 120),
    broadcast: (205, 155, 190),
    activity: (145, 180, 205),
    bell: (230, 198, 120),
    dim: (124, 129, 118),
    placeholder: (116, 121, 111),
    hint_fg: (126, 130, 119),
    find_hl_bg: (66, 72, 26),
    ansi: [
        (100, 103, 99),  // 0  black
        (255, 163, 151), // 1  red
        (123, 205, 139), // 2  green
        (225, 181, 87),  // 3  yellow
        (136, 191, 255), // 4  blue
        (230, 165, 238), // 5  magenta
        (49, 206, 213),  // 6  cyan
        (219, 222, 218), // 7  white
        (137, 140, 136), // 8  bright black
        (255, 194, 186), // 9  bright red
        (143, 226, 159), // 10 bright green
        (246, 201, 108), // 11 bright yellow
        (173, 211, 255), // 12 bright blue
        (248, 188, 255), // 13 bright magenta
        (79, 227, 234),  // 14 bright cyan
        (243, 245, 241), // 15 bright white
    ],
    dark: true,
    grain: 1.2,
    crt: None,
    modern: None,
};

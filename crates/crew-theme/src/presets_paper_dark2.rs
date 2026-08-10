//! Dark paper-family presets, the second leaf — `presets_paper.rs` sits at
//! the line cap, so new dark pages land here. Same family conventions:
//! focus-led border hierarchy, muted-but-readable ANSI slots, 1.2 grain.

use crate::Theme;

/// **Moss blotter**: a deep desaturated moss-green desk blotter with warm
/// paper-white ink — the study-lamp page of the dark family; accents lean
/// botanical (fern, spruce, lichen).
pub static MOSS_BLOTTER: Theme = Theme {
    page_bg: (17, 21, 14),
    ink: (240, 238, 225),
    text_muted: (198, 198, 178),
    term_fg: (240, 238, 225),
    term_bg: (17, 21, 14),
    // Focus-led border hierarchy, as in paper-dark.
    border_normal: (78, 88, 64),
    border_focused: (214, 218, 190),
    border_thickness: 2.5,
    legend_off: (150, 156, 130),
    accent_default: (165, 205, 140),
    status_fg: (230, 198, 120),
    broadcast: (205, 155, 190),
    activity: (145, 180, 205),
    bell: (230, 198, 120),
    dim: (128, 134, 110),
    placeholder: (116, 122, 100),
    hint_fg: (138, 144, 118),
    find_hl_bg: (66, 72, 26),
    ansi: [
        (98, 104, 86),   // 0  black -> warm-green grey
        (235, 115, 95),  // 1  red
        (150, 220, 120), // 2  green (fern)
        (225, 200, 100), // 3  yellow
        (125, 180, 225), // 4  blue
        (210, 145, 205), // 5  magenta
        (120, 215, 195), // 6  cyan (lichen)
        (222, 222, 205), // 7  white -> warm-green light grey
        (148, 152, 126), // 8  bright black
        (255, 140, 110), // 9  bright red
        (175, 240, 140), // 10 bright green
        (245, 220, 115), // 11 bright yellow
        (150, 200, 250), // 12 bright blue
        (230, 168, 228), // 13 bright magenta
        (145, 240, 215), // 14 bright cyan
        (246, 244, 232), // 15 bright white
    ],
    dark: true,
    grain: 1.2,
    crt: None,
    modern: None,
};

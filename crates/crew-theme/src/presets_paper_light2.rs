//! Light paper-family presets, the second leaf — `presets_paper_light.rs`
//! sits near the line cap, so new light pages land here. Same family
//! conventions (see presets_paper.rs): no pure black/white, deep crisp ink,
//! 1.2 newsprint grain.

use crate::Theme;

/// **Glacier bond**: a cold blue-gray bond page — good overcast north light
/// on cold-press stock — with crisp cool near-black ink; accents lean slate
/// blue. Where COLDPRESS_GRAY is strictly neutral, this page carries a
/// deliberate glacial cast.
pub static GLACIER_BOND: Theme = Theme {
    page_bg: (232, 237, 242),
    ink: (13, 15, 24),
    text_muted: (45, 48, 56),
    term_fg: (16, 19, 24),
    term_bg: (232, 237, 242),
    border_normal: (165, 169, 175),
    border_focused: (58, 74, 96),
    border_thickness: 3.0,
    legend_off: (85, 90, 96),
    accent_default: (44, 74, 110),
    status_fg: (104, 80, 24),
    broadcast: (105, 44, 90),
    activity: (36, 66, 100),
    bell: (110, 82, 20),
    dim: (102, 106, 112),
    placeholder: (110, 114, 120),
    hint_fg: (100, 105, 111),
    find_hl_bg: (224, 216, 158),
    ansi: [
        (26, 27, 29),   // 0  black
        (138, 60, 53),  // 1  red
        (25, 96, 46),   // 2  green
        (108, 80, 0),   // 3  yellow
        (35, 85, 142),  // 4  blue
        (116, 65, 122), // 5  magenta
        (0, 94, 98),    // 6  cyan
        (65, 66, 68),   // 7  white
        (90, 92, 94),   // 8  bright black
        (120, 43, 38),  // 9  bright red
        (0, 79, 31),    // 10 bright green
        (89, 65, 0),    // 11 bright yellow
        (17, 69, 124),  // 12 bright blue
        (99, 49, 105),  // 13 bright magenta
        (0, 76, 80),    // 14 bright cyan
        (28, 29, 31),   // 15 bright white
    ],
    dark: false,
    grain: 1.2,
    crt: None,
    modern: None,
};

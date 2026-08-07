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
    ink: (16, 19, 24),
    text_muted: (47, 52, 60),
    term_fg: (16, 19, 24),
    term_bg: (232, 237, 242),
    border_normal: (162, 172, 184),
    border_focused: (58, 74, 96),
    border_thickness: 3.0,
    legend_off: (88, 96, 108),
    accent_default: (44, 74, 110),
    status_fg: (104, 80, 24),
    broadcast: (105, 44, 90),
    activity: (36, 66, 100),
    bell: (110, 82, 20),
    dim: (104, 108, 116),
    placeholder: (102, 106, 114),
    hint_fg: (100, 105, 114),
    find_hl_bg: (224, 216, 158),
    ansi: [
        (24, 27, 32),   // 0  black
        (146, 40, 44),  // 1  red (rowan)
        (44, 92, 62),   // 2  green (spruce)
        (124, 96, 22),  // 3  yellow (ochre)
        (32, 74, 124),  // 4  blue (fjord)
        (98, 48, 104),  // 5  magenta (heather)
        (14, 94, 104),  // 6  cyan (ice teal)
        (62, 66, 74),   // 7  white (slate gray)
        (88, 94, 102),  // 8  bright black
        (170, 54, 54),  // 9  bright red
        (58, 110, 74),  // 10 bright green
        (130, 100, 24), // 11 bright yellow
        (44, 90, 144),  // 12 bright blue
        (118, 60, 122), // 13 bright magenta
        (20, 112, 122), // 14 bright cyan
        (26, 29, 34),   // 15 bright white (boldest ink)
    ],
    dark: false,
    grain: 1.2,
    crt: None,
};

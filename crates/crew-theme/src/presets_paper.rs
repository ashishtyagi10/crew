//! Paper-family presets: ink on paper, dark and light.

use crate::Theme;

/// High-contrast monochrome ("newspaper") dark theme — near-black/near-white
/// chrome for maximum legibility with minimal glare. Terminal ANSI output
/// keeps muted-but-readable colours so error/diff colour cues survive.
/// The default.
pub static PAPER_DARK: Theme = Theme {
    page_bg: (8, 8, 8),
    ink: (247, 247, 247),
    text_muted: (196, 196, 196),
    term_fg: (247, 247, 247),
    term_bg: (8, 8, 8),
    // Unfocused borders sit back (~3.4:1 on the page — visual parity with the
    // light theme's ~1.9:1 weight) so the FOCUSED near-white frame carries the
    // "where am I" signal instead of every card shouting equally.
    border_normal: (72, 72, 72),
    border_focused: (235, 235, 235),
    border_thickness: 2.5,
    legend_off: (140, 140, 140),
    accent_default: (240, 240, 240),
    status_fg: (235, 195, 120),
    broadcast: (200, 150, 190),
    activity: (140, 175, 210),
    bell: (235, 195, 120),
    dim: (125, 125, 125),
    placeholder: (112, 112, 112),
    hint_fg: (135, 135, 135),
    find_hl_bg: (70, 62, 20),
    ansi: [
        (95, 95, 95),    // 0  black -> neutral grey (visible on near-black)
        (235, 105, 90),  // 1  red
        (140, 220, 110), // 2  green
        (235, 200, 90),  // 3  yellow
        (120, 180, 235), // 4  blue
        (215, 140, 215), // 5  magenta
        (110, 220, 215), // 6  cyan
        (225, 225, 225), // 7  white -> neutral light grey
        (140, 140, 140), // 8  bright black
        (255, 130, 110), // 9  bright red
        (170, 240, 130), // 10 bright green
        (255, 220, 110), // 11 bright yellow
        (145, 200, 255), // 12 bright blue
        (235, 165, 235), // 13 bright magenta
        (135, 245, 235), // 14 bright cyan
        (250, 250, 250), // 15 bright white
    ],
    dark: true,
    grain: 1.0,
};

/// Warm paper "day" page — soft off-white with ink-toned output.
pub static PAPER_LIGHT: Theme = Theme {
    page_bg: (246, 243, 236),
    // Ink and every text shade run deep enough that type reads crisp on the
    // bright page (ink ≥ 16:1, muted ≥ 11:1) rather than washed-out.
    ink: (22, 20, 18),
    text_muted: (55, 51, 45),
    term_fg: (22, 20, 18),
    term_bg: (246, 243, 236),
    border_normal: (175, 166, 148),
    border_focused: (105, 97, 83),
    border_thickness: 3.0,
    legend_off: (100, 94, 83),
    accent_default: (110, 72, 38),
    status_fg: (107, 78, 26),
    broadcast: (110, 45, 88),
    activity: (40, 72, 108),
    bell: (115, 84, 20),
    dim: (105, 99, 88),
    placeholder: (118, 112, 101),
    hint_fg: (112, 106, 95),
    find_hl_bg: (235, 220, 150),
    ansi: [
        (28, 26, 23),   // 0  black
        (152, 36, 28),  // 1  red (brick)
        (58, 92, 30),   // 2  green (sage)
        (140, 96, 20),  // 3  yellow (ochre)
        (36, 74, 116),  // 4  blue (faded indigo)
        (110, 44, 96),  // 5  magenta (mauve)
        (18, 96, 92),   // 6  cyan (teal)
        (70, 66, 58),   // 7  white (warm gray)
        (92, 87, 76),   // 8  bright black
        (176, 48, 36),  // 9  bright red
        (74, 110, 40),  // 10 bright green
        (158, 108, 24), // 11 bright yellow
        (48, 92, 140),  // 12 bright blue
        (128, 58, 112), // 13 bright magenta
        (24, 114, 108), // 14 bright cyan
        (30, 28, 25),   // 15 bright white (boldest ink)
    ],
    dark: false,
    grain: 3.0,
};

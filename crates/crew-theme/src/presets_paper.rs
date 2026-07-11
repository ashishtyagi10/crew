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
    // 1.2 restores the pre-gamma-blending newsprint amplitude (was 3.0):
    // grain now modulates encoded values (v0.5.59), which reads much
    // stronger than the old linear-space pass — calibrated by measuring
    // page-luma stddev against the previous build's screenshots.
    grain: 1.2,
};

/// **Sepia dark**: dark coffee-brown paper with warm cream ink — the paper
/// family's "aged newsprint at night" page.
pub static SEPIA_DARK: Theme = Theme {
    page_bg: (24, 17, 11),
    ink: (241, 229, 205),
    text_muted: (208, 192, 164),
    term_fg: (241, 229, 205),
    term_bg: (24, 17, 11),
    // Focus-led border hierarchy, as in paper-dark.
    border_normal: (92, 74, 55),
    border_focused: (216, 192, 150),
    border_thickness: 2.5,
    legend_off: (170, 152, 124),
    accent_default: (235, 190, 120),
    status_fg: (235, 195, 120),
    broadcast: (210, 150, 180),
    activity: (150, 175, 205),
    bell: (235, 195, 120),
    dim: (140, 124, 100),
    placeholder: (128, 113, 92),
    hint_fg: (150, 134, 108),
    find_hl_bg: (80, 62, 24),
    ansi: [
        (100, 88, 72),   // 0  black -> warm grey
        (235, 110, 85),  // 1  red
        (150, 215, 110), // 2  green
        (235, 200, 95),  // 3  yellow
        (130, 180, 230), // 4  blue
        (215, 145, 205), // 5  magenta
        (120, 215, 205), // 6  cyan
        (228, 218, 200), // 7  white -> warm light grey
        (150, 136, 116), // 8  bright black
        (255, 135, 105), // 9  bright red
        (180, 235, 130), // 10 bright green
        (255, 220, 115), // 11 bright yellow
        (155, 200, 250), // 12 bright blue
        (235, 170, 225), // 13 bright magenta
        (145, 240, 225), // 14 bright cyan
        (248, 240, 225), // 15 bright white
    ],
    dark: true,
    grain: 1.0,
};

/// **Midnight ink**: deep navy page with cool off-white ink — a calm
/// blue-black newspaper.
pub static MIDNIGHT_INK: Theme = Theme {
    page_bg: (10, 14, 28),
    ink: (232, 238, 248),
    text_muted: (185, 196, 215),
    term_fg: (232, 238, 248),
    term_bg: (10, 14, 28),
    // Focus-led border hierarchy, as in paper-dark.
    border_normal: (66, 76, 100),
    border_focused: (200, 214, 235),
    border_thickness: 2.5,
    legend_off: (140, 152, 175),
    accent_default: (150, 190, 245),
    status_fg: (235, 200, 120),
    broadcast: (200, 155, 215),
    activity: (130, 180, 225),
    bell: (235, 200, 120),
    dim: (110, 120, 140),
    placeholder: (100, 110, 130),
    hint_fg: (120, 131, 152),
    find_hl_bg: (50, 62, 100),
    ansi: [
        (90, 96, 110),   // 0  black -> cool grey
        (240, 110, 100), // 1  red
        (135, 215, 125), // 2  green
        (230, 200, 100), // 3  yellow
        (115, 175, 240), // 4  blue
        (205, 145, 220), // 5  magenta
        (105, 215, 220), // 6  cyan
        (220, 226, 236), // 7  white -> cool light grey
        (135, 143, 158), // 8  bright black
        (255, 135, 120), // 9  bright red
        (165, 235, 145), // 10 bright green
        (250, 220, 120), // 11 bright yellow
        (140, 195, 255), // 12 bright blue
        (230, 168, 240), // 13 bright magenta
        (130, 240, 240), // 14 bright cyan
        (245, 248, 252), // 15 bright white
    ],
    dark: true,
    grain: 1.0,
};

/// **Graphite**: neutral charcoal page with soft white ink — a gentler,
/// lower-glare paper-dark.
pub static GRAPHITE: Theme = Theme {
    page_bg: (28, 28, 30),
    ink: (226, 226, 228),
    text_muted: (183, 183, 186),
    term_fg: (226, 226, 228),
    term_bg: (28, 28, 30),
    // Focus-led border hierarchy, as in paper-dark.
    border_normal: (85, 85, 88),
    border_focused: (215, 215, 218),
    border_thickness: 2.5,
    legend_off: (150, 150, 154),
    accent_default: (222, 222, 225),
    status_fg: (230, 195, 125),
    broadcast: (198, 152, 188),
    activity: (142, 175, 208),
    bell: (230, 195, 125),
    dim: (130, 130, 134),
    placeholder: (120, 120, 124),
    hint_fg: (140, 140, 144),
    find_hl_bg: (75, 68, 28),
    ansi: [
        (110, 110, 113), // 0  black -> mid grey
        (235, 110, 95),  // 1  red
        (145, 220, 115), // 2  green
        (235, 200, 95),  // 3  yellow
        (125, 182, 235), // 4  blue
        (215, 145, 215), // 5  magenta
        (115, 220, 215), // 6  cyan
        (222, 222, 225), // 7  white -> light grey
        (145, 145, 149), // 8  bright black
        (255, 135, 115), // 9  bright red
        (175, 240, 135), // 10 bright green
        (255, 220, 115), // 11 bright yellow
        (150, 202, 255), // 12 bright blue
        (235, 168, 235), // 13 bright magenta
        (140, 245, 235), // 14 bright cyan
        (246, 246, 248), // 15 bright white
    ],
    dark: true,
    grain: 1.0,
};

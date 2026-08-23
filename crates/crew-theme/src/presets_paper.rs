//! Paper-family presets: ink on paper, dark and light.

use crate::Theme;

/// High-contrast monochrome ("newspaper") dark theme — warm near-black/near-white
/// chrome for maximum legibility with minimal glare. The page leans warm
/// charcoal since the 2026-07-24 retune. Terminal ANSI output keeps
/// muted-but-readable colours so error/diff colour cues survive. The default.
pub static PAPER_DARK: Theme = Theme {
    page_bg: (12, 8, 5),
    ink: (232, 232, 232),
    text_muted: (197, 194, 193),
    term_fg: (247, 247, 247),
    term_bg: (12, 8, 5),
    // Unfocused borders sit back (~3.4:1 on the page — visual parity with the
    // light theme's ~1.9:1 weight) so the FOCUSED near-white frame carries the
    // "where am I" signal instead of every card shouting equally.
    border_normal: (71, 66, 61),
    border_focused: (235, 235, 235),
    border_thickness: 2.5,
    legend_off: (144, 139, 136),
    accent_default: (240, 240, 240),
    status_fg: (235, 195, 120),
    broadcast: (200, 150, 190),
    activity: (140, 175, 210),
    bell: (235, 195, 120),
    dim: (126, 121, 118),
    placeholder: (118, 113, 109),
    hint_fg: (127, 122, 119),
    find_hl_bg: (70, 62, 20),
    ansi: [
        (99, 97, 94),    // 0  black
        (253, 153, 140), // 1  red
        (116, 198, 132), // 2  green
        (217, 174, 79),  // 3  yellow
        (123, 184, 255), // 4  blue
        (223, 158, 230), // 5  magenta
        (35, 199, 205),  // 6  cyan
        (215, 213, 210), // 7  white
        (135, 133, 130), // 8  bright black
        (255, 184, 174), // 9  bright red
        (136, 219, 151), // 10 bright green
        (238, 194, 100), // 11 bright yellow
        (161, 204, 255), // 12 bright blue
        (244, 178, 251), // 13 bright magenta
        (69, 220, 226),  // 14 bright cyan
        (238, 235, 232), // 15 bright white
    ],
    dark: true,
    grain: 1.2,
    crt: None,
    modern: None,
};

/// Warm paper "day" page — soft off-white with ink-toned output.
pub static PAPER_LIGHT: Theme = Theme {
    page_bg: (246, 243, 236),
    // Ink and every text shade run deep enough that type reads crisp on the
    // bright page (ink ≥ 16:1, muted ≥ 11:1) rather than washed-out.
    ink: (26, 22, 20),
    text_muted: (56, 51, 48),
    term_fg: (22, 20, 18),
    term_bg: (246, 243, 236),
    border_normal: (177, 173, 167),
    border_focused: (105, 97, 83),
    border_thickness: 3.0,
    legend_off: (96, 93, 88),
    accent_default: (110, 72, 38),
    status_fg: (107, 78, 26),
    broadcast: (110, 45, 88),
    activity: (40, 72, 108),
    bell: (115, 84, 20),
    dim: (113, 109, 104),
    placeholder: (121, 117, 112),
    hint_fg: (112, 108, 103),
    find_hl_bg: (235, 220, 150),
    ansi: [
        (33, 32, 31),   // 0  black
        (142, 64, 57),  // 1  red
        (30, 100, 50),  // 2  green
        (113, 83, 0),   // 3  yellow
        (39, 89, 146),  // 4  blue
        (120, 69, 127), // 5  magenta
        (0, 98, 102),   // 6  cyan
        (71, 70, 68),   // 7  white
        (97, 95, 93),   // 8  bright black
        (124, 47, 42),  // 9  bright red
        (5, 83, 34),    // 10 bright green
        (93, 68, 0),    // 11 bright yellow
        (21, 72, 128),  // 12 bright blue
        (103, 53, 110), // 13 bright magenta
        (0, 80, 84),    // 14 bright cyan
        (35, 34, 32),   // 15 bright white
    ],
    dark: false,
    // 1.2 restores the pre-gamma-blending newsprint amplitude (was 3.0):
    // grain now modulates encoded values (v0.5.58), which reads much
    // stronger than the old linear-space pass — calibrated by measuring
    // page-luma stddev against the previous build's screenshots.
    grain: 1.2,
    crt: None,
    modern: None,
};

/// **Sepia dark**: dark coffee-brown paper with warm cream ink — the paper
/// family's "aged newsprint at night" page.
pub static SEPIA_DARK: Theme = Theme {
    page_bg: (24, 17, 11),
    ink: (249, 238, 213),
    text_muted: (211, 199, 180),
    term_fg: (241, 229, 205),
    term_bg: (24, 17, 11),
    // Focus-led border hierarchy, as in paper-dark.
    border_normal: (78, 70, 62),
    border_focused: (216, 192, 150),
    border_thickness: 2.5,
    legend_off: (154, 143, 131),
    accent_default: (235, 190, 120),
    status_fg: (235, 195, 120),
    broadcast: (210, 150, 180),
    activity: (150, 175, 205),
    bell: (235, 195, 120),
    dim: (135, 125, 114),
    placeholder: (127, 116, 106),
    hint_fg: (137, 126, 115),
    find_hl_bg: (80, 62, 24),
    ansi: [
        (104, 101, 97),  // 0  black
        (255, 161, 150), // 1  red
        (122, 204, 138), // 2  green
        (223, 180, 85),  // 3  yellow
        (134, 190, 255), // 4  blue
        (229, 163, 237), // 5  magenta
        (46, 205, 211),  // 6  cyan
        (224, 219, 216), // 7  white
        (141, 138, 134), // 8  bright black
        (255, 193, 184), // 9  bright red
        (142, 225, 158), // 10 bright green
        (244, 200, 106), // 11 bright yellow
        (171, 210, 255), // 12 bright blue
        (248, 185, 255), // 13 bright magenta
        (77, 226, 232),  // 14 bright cyan
        (247, 242, 239), // 15 bright white
    ],
    dark: true,
    grain: 1.2,
    crt: None,
    modern: None,
};

/// **Midnight ink**: warm slate-charcoal page with cool off-white ink — a
/// calm newspaper that trades the old blue-black cast for warmth while
/// keeping the ink itself cool.
pub static MIDNIGHT_INK: Theme = Theme {
    page_bg: (16, 14, 12),
    ink: (230, 236, 244),
    text_muted: (200, 196, 203),
    term_fg: (232, 238, 248),
    term_bg: (16, 14, 12),
    // Focus-led border hierarchy, as in paper-dark.
    border_normal: (72, 68, 66),
    border_focused: (200, 214, 235),
    border_thickness: 2.5,
    legend_off: (147, 141, 141),
    accent_default: (150, 190, 245),
    status_fg: (235, 200, 120),
    broadcast: (200, 155, 215),
    activity: (130, 180, 225),
    bell: (235, 200, 120),
    dim: (128, 123, 122),
    placeholder: (120, 115, 113),
    hint_fg: (130, 125, 124),
    find_hl_bg: (50, 62, 100),
    ansi: [
        (100, 99, 97),   // 0  black
        (255, 157, 145), // 1  red
        (119, 201, 135), // 2  green
        (221, 177, 82),  // 3  yellow
        (129, 187, 255), // 4  blue
        (226, 161, 234), // 5  magenta
        (42, 202, 209),  // 6  cyan
        (218, 217, 215), // 7  white
        (137, 136, 134), // 8  bright black
        (255, 189, 180), // 9  bright red
        (139, 222, 154), // 10 bright green
        (242, 197, 103), // 11 bright yellow
        (166, 207, 255), // 12 bright blue
        (247, 181, 255), // 13 bright magenta
        (74, 223, 230),  // 14 bright cyan
        (241, 239, 238), // 15 bright white
    ],
    dark: true,
    grain: 1.2,
    crt: None,
    modern: None,
};

/// **Graphite**: warm charcoal page with soft white ink — a gentler,
/// lower-glare paper-dark.
pub static GRAPHITE: Theme = Theme {
    page_bg: (32, 28, 27),
    ink: (241, 241, 243),
    text_muted: (212, 210, 212),
    term_fg: (226, 226, 228),
    term_bg: (32, 28, 27),
    // Focus-led border hierarchy, as in paper-dark.
    border_normal: (81, 76, 76),
    border_focused: (215, 215, 218),
    border_thickness: 2.5,
    legend_off: (156, 152, 153),
    accent_default: (222, 222, 225),
    status_fg: (230, 195, 125),
    broadcast: (198, 152, 188),
    activity: (142, 175, 208),
    bell: (230, 195, 125),
    dim: (137, 133, 133),
    placeholder: (129, 124, 124),
    hint_fg: (139, 134, 134),
    find_hl_bg: (75, 68, 28),
    ansi: [
        (110, 108, 107), // 0  black
        (255, 176, 165), // 1  red
        (132, 214, 147), // 2  green
        (233, 190, 95),  // 3  yellow
        (152, 199, 255), // 4  blue
        (239, 173, 246), // 5  magenta
        (62, 215, 221),  // 6  cyan
        (233, 230, 230), // 7  white
        (148, 145, 145), // 8  bright black
        (255, 206, 199), // 9  bright red
        (152, 235, 167), // 10 bright green
        (254, 211, 116), // 11 bright yellow
        (188, 219, 255), // 12 bright blue
        (250, 201, 255), // 13 bright magenta
        (89, 236, 242),  // 14 bright cyan
        (255, 255, 254), // 15 bright white
    ],
    dark: true,
    grain: 1.2,
    crt: None,
    modern: None,
};

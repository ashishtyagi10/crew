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
    find_hl_bg: (225, 195, 110),
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

/// **Salmon broadsheet**: Financial-Times-style salmon-pink page with cool
/// near-black ink; accents lean navy/teal.
pub static SALMON_BROADSHEET: Theme = Theme {
    page_bg: (250, 231, 215),
    ink: (12, 12, 20),
    text_muted: (54, 43, 52),
    term_fg: (12, 13, 18),
    term_bg: (250, 231, 215),
    border_normal: (183, 163, 151),
    border_focused: (45, 50, 75),
    border_thickness: 3.0,
    legend_off: (103, 83, 84),
    accent_default: (30, 55, 95),
    status_fg: (110, 82, 20),
    broadcast: (115, 40, 85),
    activity: (35, 70, 105),
    bell: (118, 86, 15),
    dim: (120, 100, 97),
    placeholder: (128, 108, 103),
    hint_fg: (119, 98, 95),
    find_hl_bg: (232, 205, 140),
    ansi: [
        (30, 24, 20),   // 0  black
        (136, 59, 52),  // 1  red
        (24, 95, 45),   // 2  green
        (107, 78, 0),   // 3  yellow
        (34, 84, 141),  // 4  blue
        (114, 64, 120), // 5  magenta
        (0, 92, 96),    // 6  cyan
        (70, 64, 58),   // 7  white
        (96, 89, 84),   // 8  bright black
        (118, 42, 37),  // 9  bright red
        (0, 78, 30),    // 10 bright green
        (88, 63, 0),    // 11 bright yellow
        (15, 68, 123),  // 12 bright blue
        (97, 48, 103),  // 13 bright magenta
        (0, 75, 78),    // 14 bright cyan
        (32, 26, 22),   // 15 bright white
    ],
    dark: false,
    grain: 1.2,
    crt: None,
    modern: None,
};

/// **Coldpress gray**: cool pale-gray page with near-black neutral ink — the
/// light twin of GRAPHITE; the lowest-glare option.
pub static COLDPRESS_GRAY: Theme = Theme {
    page_bg: (238, 238, 240),
    ink: (17, 17, 22),
    text_muted: (48, 49, 52),
    term_fg: (18, 18, 19),
    term_bg: (238, 238, 240),
    border_normal: (170, 170, 172),
    border_focused: (96, 96, 100),
    border_thickness: 3.0,
    legend_off: (90, 90, 93),
    accent_default: (62, 64, 72),
    status_fg: (108, 80, 25),
    broadcast: (108, 45, 92),
    activity: (38, 68, 102),
    bell: (112, 84, 20),
    dim: (106, 107, 109),
    placeholder: (114, 114, 117),
    hint_fg: (105, 105, 108),
    find_hl_bg: (230, 222, 160),
    ansi: [
        (28, 28, 29),   // 0  black
        (139, 61, 54),  // 1  red
        (26, 97, 47),   // 2  green
        (110, 80, 0),   // 3  yellow
        (36, 86, 143),  // 4  blue
        (117, 66, 123), // 5  magenta
        (0, 95, 99),    // 6  cyan
        (67, 67, 68),   // 7  white
        (92, 92, 93),   // 8  bright black
        (121, 44, 39),  // 9  bright red
        (0, 81, 31),    // 10 bright green
        (90, 65, 0),    // 11 bright yellow
        (18, 69, 125),  // 12 bright blue
        (100, 50, 106), // 13 bright magenta
        (0, 77, 81),    // 14 bright cyan
        (30, 30, 31),   // 15 bright white
    ],
    dark: false,
    grain: 1.2,
    crt: None,
    modern: None,
};

/// **Ivory ledger**: slightly yellow ivory page with green-black ink — an
/// old accounting-ledger feel; accents lean deep green.
pub static IVORY_LEDGER: Theme = Theme {
    page_bg: (244, 239, 214),
    ink: (15, 19, 12),
    text_muted: (47, 50, 39),
    term_fg: (15, 19, 12),
    term_bg: (244, 239, 214),
    border_normal: (175, 171, 149),
    border_focused: (90, 96, 70),
    border_thickness: 3.0,
    legend_off: (91, 92, 74),
    accent_default: (30, 80, 40),
    status_fg: (112, 84, 20),
    broadcast: (108, 44, 90),
    activity: (36, 70, 104),
    bell: (116, 86, 18),
    dim: (109, 108, 89),
    placeholder: (117, 116, 97),
    hint_fg: (107, 106, 87),
    find_hl_bg: (228, 214, 130),
    ansi: [
        (30, 29, 22),   // 0  black
        (139, 61, 54),  // 1  red
        (27, 98, 48),   // 2  green
        (110, 80, 0),   // 3  yellow
        (36, 87, 143),  // 4  blue
        (117, 66, 123), // 5  magenta
        (0, 95, 99),    // 6  cyan
        (69, 67, 60),   // 7  white
        (95, 93, 85),   // 8  bright black
        (121, 44, 39),  // 9  bright red
        (1, 81, 32),    // 10 bright green
        (90, 65, 0),    // 11 bright yellow
        (18, 70, 125),  // 12 bright blue
        (100, 50, 106), // 13 bright magenta
        (0, 77, 81),    // 14 bright cyan
        (32, 31, 23),   // 15 bright white
    ],
    dark: false,
    grain: 1.2,
    crt: None,
    modern: None,
};

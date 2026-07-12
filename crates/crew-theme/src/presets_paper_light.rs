//! Light paper-family presets: four newspaper pages (see presets_paper.rs for the family conventions).

use crate::Theme;

/// **Sepia light**: warm aged-newsprint cream page with deep brown-black
/// ink — the light twin of SEPIA_DARK, echoing its warm gold accent
/// character.
pub static SEPIA_LIGHT: Theme = Theme {
    page_bg: (245, 235, 205),
    ink: (20, 13, 8),
    text_muted: (59, 45, 29),
    term_fg: (20, 13, 8),
    term_bg: (245, 235, 205),
    border_normal: (186, 160, 118),
    border_focused: (120, 90, 50),
    border_thickness: 3.0,
    legend_off: (108, 84, 52),
    accent_default: (150, 90, 20),
    status_fg: (140, 90, 10),
    broadcast: (140, 50, 90),
    activity: (50, 80, 120),
    bell: (146, 93, 15),
    dim: (120, 100, 80),
    placeholder: (120, 101, 78),
    hint_fg: (121, 101, 77),
    find_hl_bg: (225, 195, 110),
    ansi: [
        (40, 28, 18),   // 0  black
        (162, 40, 26),  // 1  red (brick)
        (70, 95, 28),   // 2  green (sage)
        (142, 95, 14),  // 3  yellow (ochre)
        (45, 80, 120),  // 4  blue (faded indigo)
        (120, 48, 100), // 5  magenta (mauve)
        (20, 100, 95),  // 6  cyan (teal)
        (80, 68, 50),   // 7  white (warm gray)
        (100, 85, 62),  // 8  bright black
        (185, 55, 35),  // 9  bright red
        (85, 115, 35),  // 10 bright green
        (140, 95, 17),  // 11 bright yellow
        (55, 95, 140),  // 12 bright blue
        (138, 60, 115), // 13 bright magenta
        (28, 118, 112), // 14 bright cyan
        (42, 30, 20),   // 15 bright white (boldest ink)
    ],
    dark: false,
    grain: 1.2,
};

/// **Salmon broadsheet**: Financial-Times-style salmon-pink page with cool
/// near-black ink; accents lean navy/teal.
pub static SALMON_BROADSHEET: Theme = Theme {
    page_bg: (250, 231, 215),
    ink: (12, 13, 18),
    text_muted: (46, 46, 53),
    term_fg: (12, 13, 18),
    term_bg: (250, 231, 215),
    border_normal: (178, 152, 130),
    border_focused: (45, 50, 75),
    border_thickness: 3.0,
    legend_off: (95, 86, 90),
    accent_default: (30, 55, 95),
    status_fg: (110, 82, 20),
    broadcast: (115, 40, 85),
    activity: (35, 70, 105),
    bell: (118, 86, 15),
    dim: (110, 95, 90),
    placeholder: (114, 101, 96),
    hint_fg: (116, 102, 96),
    find_hl_bg: (232, 205, 140),
    ansi: [
        (24, 22, 26),   // 0  black
        (150, 38, 40),  // 1  red
        (45, 90, 55),   // 2  green (forest)
        (130, 95, 20),  // 3  yellow (ochre)
        (30, 70, 115),  // 4  blue (navy)
        (100, 45, 95),  // 5  magenta (plum)
        (15, 95, 98),   // 6  cyan (teal)
        (60, 58, 64),   // 7  white (cool gray)
        (86, 80, 86),   // 8  bright black
        (172, 50, 48),  // 9  bright red
        (58, 108, 66),  // 10 bright green
        (133, 98, 22),  // 11 bright yellow
        (40, 88, 140),  // 12 bright blue
        (118, 55, 112), // 13 bright magenta
        (20, 115, 118), // 14 bright cyan
        (26, 24, 28),   // 15 bright white (boldest ink)
    ],
    dark: false,
    grain: 1.2,
};

/// **Coldpress gray**: cool pale-gray page with near-black neutral ink — the
/// light twin of GRAPHITE; the lowest-glare option.
pub static COLDPRESS_GRAY: Theme = Theme {
    page_bg: (238, 238, 240),
    ink: (18, 18, 19),
    text_muted: (49, 49, 52),
    term_fg: (18, 18, 19),
    term_bg: (238, 238, 240),
    border_normal: (172, 172, 176),
    border_focused: (96, 96, 100),
    border_thickness: 3.0,
    legend_off: (96, 96, 100),
    accent_default: (62, 64, 72),
    status_fg: (108, 80, 25),
    broadcast: (108, 45, 92),
    activity: (38, 68, 102),
    bell: (112, 84, 20),
    dim: (108, 108, 112),
    placeholder: (106, 106, 109),
    hint_fg: (106, 106, 109),
    find_hl_bg: (230, 222, 160),
    ansi: [
        (26, 26, 28),   // 0  black
        (148, 42, 40),  // 1  red
        (52, 92, 52),   // 2  green
        (128, 98, 20),  // 3  yellow (ochre)
        (34, 78, 120),  // 4  blue
        (104, 46, 98),  // 5  magenta
        (16, 98, 98),   // 6  cyan (teal)
        (66, 66, 70),   // 7  white (cool gray)
        (92, 92, 96),   // 8  bright black
        (172, 55, 50),  // 9  bright red
        (66, 112, 64),  // 10 bright green
        (134, 101, 23), // 11 bright yellow
        (46, 94, 142),  // 12 bright blue
        (124, 58, 116), // 13 bright magenta
        (22, 116, 116), // 14 bright cyan
        (28, 28, 30),   // 15 bright white (boldest ink)
    ],
    dark: false,
    grain: 1.2,
};

/// **Ivory ledger**: slightly yellow ivory page with green-black ink — an
/// old accounting-ledger feel; accents lean deep green.
pub static IVORY_LEDGER: Theme = Theme {
    page_bg: (244, 239, 214),
    ink: (15, 19, 12),
    text_muted: (46, 51, 41),
    term_fg: (15, 19, 12),
    term_bg: (244, 239, 214),
    border_normal: (178, 172, 138),
    border_focused: (90, 96, 70),
    border_thickness: 3.0,
    legend_off: (92, 90, 70),
    accent_default: (30, 80, 40),
    status_fg: (112, 84, 20),
    broadcast: (108, 44, 90),
    activity: (36, 70, 104),
    bell: (116, 86, 18),
    dim: (108, 104, 84),
    placeholder: (110, 106, 88),
    hint_fg: (110, 107, 87),
    find_hl_bg: (228, 214, 130),
    ansi: [
        (24, 28, 18),   // 0  black
        (150, 40, 30),  // 1  red
        (35, 95, 42),   // 2  green (ledger green)
        (138, 100, 18), // 3  yellow (ochre)
        (36, 74, 112),  // 4  blue
        (108, 46, 92),  // 5  magenta
        (16, 98, 88),   // 6  cyan (teal)
        (68, 68, 54),   // 7  white (warm-green gray)
        (92, 90, 70),   // 8  bright black
        (174, 52, 38),  // 9  bright red
        (44, 116, 52),  // 10 bright green
        (139, 99, 18),  // 11 bright yellow
        (46, 92, 138),  // 12 bright blue
        (126, 58, 110), // 13 bright magenta
        (20, 114, 104), // 14 bright cyan
        (18, 22, 14),   // 15 bright white (boldest ink)
    ],
    dark: false,
    grain: 1.2,
};

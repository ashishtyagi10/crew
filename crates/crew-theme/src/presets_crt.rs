//! CRT-family presets: neon phosphor tubes.

use crate::Theme;

/// **Neon green phosphor** (P1, electrified): hot Tron-grid green on a
/// near-black tube, with a monochrome-green ANSI palette (brightness tiers,
/// faint hue tilts) for that single-gun terminal look. The paper-grain pass
/// reads as a subtle glow.
pub static CRT_GREEN: Theme = Theme {
    page_bg: (3, 10, 5),
    ink: (0, 255, 102),
    text_muted: (0, 204, 82),
    term_fg: (0, 255, 102),
    term_bg: (3, 10, 5),
    // Unfocused borders sit back (matching paper-dark's focus-led hierarchy)
    // so the bright phosphor frame alone says which pane is live.
    border_normal: (0, 88, 42),
    border_focused: (0, 255, 140),
    border_thickness: 2.5,
    legend_off: (0, 160, 70),
    accent_default: (64, 255, 160),
    status_fg: (190, 255, 80),
    broadcast: (150, 255, 150),
    activity: (0, 230, 120),
    bell: (200, 255, 90),
    dim: (0, 110, 55),
    placeholder: (0, 135, 60),
    hint_fg: (0, 150, 66),
    find_hl_bg: (10, 70, 30),
    ansi: [
        (10, 45, 20),    // 0  black
        (170, 255, 70),  // 1  red
        (0, 255, 102),   // 2  green
        (200, 255, 80),  // 3  yellow
        (0, 230, 170),   // 4  blue
        (130, 255, 150), // 5  magenta
        (0, 255, 200),   // 6  cyan
        (170, 255, 190), // 7  white
        (0, 140, 70),    // 8  bright black
        (200, 255, 100), // 9  bright red
        (80, 255, 130),  // 10 bright green
        (230, 255, 110), // 11 bright yellow
        (60, 255, 200),  // 12 bright blue
        (170, 255, 180), // 13 bright magenta
        (100, 255, 230), // 14 bright cyan
        (210, 255, 220), // 15 bright white
    ],
};

/// **Neon amber phosphor** (P3, electrified): saturated Tron-orange amber on a
/// near-black tube — the warm counterpart of the green grid.
pub static CRT_AMBER: Theme = Theme {
    page_bg: (14, 8, 2),
    ink: (255, 184, 0),
    text_muted: (226, 148, 0),
    term_fg: (255, 184, 0),
    term_bg: (14, 8, 2),
    // Unfocused borders sit back (focus-led hierarchy, as in paper-dark).
    border_normal: (112, 70, 16),
    border_focused: (255, 170, 40),
    border_thickness: 2.5,
    legend_off: (180, 115, 20),
    accent_default: (255, 210, 60),
    status_fg: (255, 200, 70),
    broadcast: (255, 170, 110),
    activity: (255, 170, 50),
    bell: (255, 190, 40),
    dim: (130, 85, 25),
    placeholder: (155, 100, 25),
    hint_fg: (172, 110, 25),
    find_hl_bg: (75, 48, 10),
    ansi: [
        (60, 35, 10),    // 0  black
        (255, 120, 40),  // 1  red
        (240, 200, 40),  // 2  green
        (255, 200, 30),  // 3  yellow
        (255, 160, 90),  // 4  blue
        (255, 140, 90),  // 5  magenta
        (250, 190, 110), // 6  cyan
        (255, 205, 120), // 7  white
        (150, 95, 35),   // 8  bright black
        (255, 140, 60),  // 9  bright red
        (255, 220, 60),  // 10 bright green
        (255, 215, 70),  // 11 bright yellow
        (255, 180, 110), // 12 bright blue
        (255, 160, 110), // 13 bright magenta
        (255, 210, 140), // 14 bright cyan
        (255, 225, 160), // 15 bright white
    ],
};

/// **Neon blue phosphor** (electrified): Tron light-cycle cyan on a
/// near-black tube — electric edge-glow blues, the coolest of the three grids.
pub static CRT_BLUE: Theme = Theme {
    page_bg: (2, 8, 18),
    ink: (0, 229, 255),
    text_muted: (0, 182, 214),
    term_fg: (0, 229, 255),
    term_bg: (2, 8, 18),
    // Unfocused borders sit back (focus-led hierarchy, as in paper-dark).
    border_normal: (0, 78, 110),
    border_focused: (0, 215, 255),
    border_thickness: 2.5,
    legend_off: (0, 145, 180),
    accent_default: (120, 255, 255),
    status_fg: (150, 230, 255),
    broadcast: (170, 180, 255),
    activity: (0, 200, 240),
    bell: (170, 220, 255),
    dim: (0, 105, 140),
    placeholder: (0, 122, 155),
    hint_fg: (0, 138, 172),
    find_hl_bg: (10, 45, 75),
    ansi: [
        (20, 50, 75),    // 0  black
        (150, 170, 255), // 1  red
        (0, 255, 220),   // 2  green
        (140, 220, 255), // 3  yellow
        (60, 160, 255),  // 4  blue
        (150, 150, 255), // 5  magenta
        (0, 240, 255),   // 6  cyan
        (170, 225, 255), // 7  white
        (0, 120, 170),   // 8  bright black
        (180, 190, 255), // 9  bright red
        (60, 255, 235),  // 10 bright green
        (170, 235, 255), // 11 bright yellow
        (90, 190, 255),  // 12 bright blue
        (180, 170, 255), // 13 bright magenta
        (110, 250, 255), // 14 bright cyan
        (200, 240, 255), // 15 bright white
    ],
};

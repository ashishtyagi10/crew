//! NEON presets: a neon scoreboard, dark and light (the user, 2026-10-22:
//! "generate a new crew theme similar to this video, you can have both light
//! and dark variation on it"). The video is a head-to-head scoreboard: deep
//! indigo cards outlined in thin neon lines, a yellow headline, cyan and
//! violet for the two sides, hot pink for a miss and green for a hit.
//!
//! Opaque, like the Ink Reader pages: no glass, no glow, no gradient, because
//! the scoreboard has none. The colour is in the lines and the marks. The
//! text ladder and the ANSI slots are derived like every other palette's
//! ([`crate::ramp`], [`crate::ansi`]); what each preset declares is its page,
//! its ink's hue, and the neon roles.

use crate::Theme;

/// Neon, dark: the video itself. A deep indigo page (the scoreboard's card,
/// `#120C2A`), lavender-white words, and the headline's yellow for where you
/// are: the focused frame and the accent. Cyan is progress, pink rings the
/// bell, green is a live pane and violet the broadcast — the video's two
/// sides, its misses and its hits.
pub static NEON_DARK: Theme = Theme {
    page_bg: (18, 12, 42),
    ink: (238, 236, 250),
    text_muted: (200, 197, 220),
    term_fg: (238, 236, 250),
    term_bg: (18, 12, 42),
    border_normal: (96, 93, 126),
    border_focused: (245, 225, 0),
    border_thickness: 2.5,
    legend_off: (144, 141, 171),
    accent_default: (245, 225, 0),
    status_fg: (24, 216, 238),
    broadcast: (176, 128, 245),
    activity: (52, 248, 132),
    bell: (255, 96, 172),
    dim: (125, 122, 154),
    placeholder: (117, 114, 146),
    hint_fg: (127, 124, 156),
    find_hl_bg: (86, 74, 0),
    ansi: [
        (100, 99, 112),  // 0  black
        (255, 160, 148), // 1  red
        (121, 203, 137), // 2  green
        (222, 179, 84),  // 3  yellow
        (132, 189, 255), // 4  blue
        (228, 162, 236), // 5  magenta
        (45, 204, 210),  // 6  cyan
        (218, 218, 232), // 7  white
        (137, 136, 149), // 8  bright black
        (255, 192, 183), // 9  bright red
        (141, 224, 157), // 10 bright green
        (243, 199, 105), // 11 bright yellow
        (169, 209, 255), // 12 bright blue
        (247, 184, 255), // 13 bright magenta
        (76, 225, 231),  // 14 bright cyan
        (241, 241, 254), // 15 bright white
    ],
    dark: true,
    crt: None,
    modern: None,
    liquid: None,
};

/// Neon, light: the scoreboard printed on a pale lavender page. Yellow cannot
/// carry a line on white, so the focused frame takes the video's hot pink;
/// the accent is its cyan and the broadcast its violet, deepened until they
/// read on the page, and the words are a deep indigo.
pub static NEON_LIGHT: Theme = Theme {
    page_bg: (248, 246, 254),
    ink: (27, 20, 58),
    text_muted: (56, 50, 80),
    term_fg: (27, 20, 58),
    term_bg: (248, 246, 254),
    border_normal: (144, 142, 154),
    border_focused: (214, 18, 128),
    border_thickness: 2.5,
    legend_off: (97, 93, 112),
    accent_default: (214, 18, 128),
    status_fg: (0, 118, 150),
    broadcast: (112, 48, 220),
    activity: (0, 112, 56),
    bell: (184, 56, 20),
    dim: (114, 110, 127),
    placeholder: (122, 118, 133),
    hint_fg: (112, 108, 125),
    find_hl_bg: (250, 164, 210),
    ansi: [
        (36, 35, 37),   // 0  black
        (144, 67, 59),  // 1  red
        (35, 104, 53),  // 2  green
        (116, 85, 0),   // 3  yellow
        (41, 92, 149),  // 4  blue
        (123, 71, 129), // 5  magenta
        (0, 101, 104),  // 6  cyan
        (73, 73, 73),   // 7  white
        (98, 97, 100),  // 8  bright black
        (126, 50, 43),  // 9  bright red
        (11, 86, 36),   // 10 bright green
        (96, 70, 0),    // 11 bright yellow
        (23, 75, 131),  // 12 bright blue
        (106, 55, 112), // 13 bright magenta
        (0, 83, 86),    // 14 bright cyan
        (37, 36, 39),   // 15 bright white
    ],
    dark: false,
    crt: None,
    modern: None,
    liquid: None,
};

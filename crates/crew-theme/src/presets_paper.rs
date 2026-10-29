//! Paper-family presets: Ink Reader, the e-reader look, dark and light.

use crate::Theme;

/// Ink Reader, dark: an e-reader's black theme (the user, 2026-10-16: "the
/// dark theme to dark Ink Reader look"). A black page, white-grey words and
/// grey lines, the focused frame white. Opaque: no glass, no glow, no text
/// shadow, no gradient, the page as paper. Colour stays where it means
/// something: the ANSI output and the status roles. The accent is the
/// page's brightest white, and a selection is black on it, the e-reader's
/// inverse highlight.
pub static PAPER_DARK: Theme = Theme {
    page_bg: (10, 9, 8),
    ink: (232, 232, 232),
    text_muted: (195, 195, 194),
    term_fg: (232, 232, 232),
    term_bg: (10, 9, 8),
    // Unfocused lines are a grey of the ink at 3:1, a line you can see; the
    // focused frame is the ink itself, the way an e-reader marks where you are.
    border_normal: (94, 93, 91),
    border_focused: (232, 232, 232),
    border_thickness: 2.5,
    legend_off: (141, 140, 139),
    accent_default: (242, 242, 242),
    status_fg: (235, 195, 120),
    broadcast: (200, 150, 190),
    activity: (140, 175, 210),
    bell: (254, 184, 174),
    dim: (123, 122, 121),
    placeholder: (115, 113, 113),
    hint_fg: (124, 123, 122),
    find_hl_bg: (70, 62, 20),
    ansi: [
        (97, 97, 96),    // 0  black
        (253, 153, 140), // 1  red
        (116, 198, 132), // 2  green
        (217, 174, 79),  // 3  yellow
        (123, 184, 255), // 4  blue
        (223, 158, 230), // 5  magenta
        (35, 199, 205),  // 6  cyan
        (214, 214, 213), // 7  white
        (134, 133, 132), // 8  bright black
        (255, 184, 174), // 9  bright red
        (136, 219, 151), // 10 bright green
        (238, 194, 100), // 11 bright yellow
        (161, 204, 255), // 12 bright blue
        (244, 178, 251), // 13 bright magenta
        (69, 220, 226),  // 14 bright cyan
        (236, 236, 235), // 15 bright white
    ],
    dark: true,
    crt: None,
    // No poles: the focused frame is a plain line, the meters a grey of the
    // ink. Flat print, nothing drifting or glowing.
    modern: None,
    liquid: None,
    syntax: crate::SyntaxSlots::HOUSE,
    selection: None,
};

/// Ink Reader, light: an e-reader's white theme, the look of the Kindle app
/// (the user, 2026-10-16: "white background with black lines"). A white
/// page, near-black words and black lines; grey is the only second ink.
/// Opaque: no glass, no glow, no text shadow, no gradient. Colour is printed,
/// not lit: deep inks for the terminal's ANSI output and the status roles,
/// and a highlighter yellow under a search match. The accent is a grey of
/// the ink, and a selection is white on it, the e-reader's inverse highlight.
pub static PAPER_LIGHT: Theme = Theme {
    page_bg: (250, 250, 248),
    ink: (28, 28, 28),
    text_muted: (56, 56, 56),
    term_fg: (17, 17, 17),
    term_bg: (250, 250, 248),
    border_normal: (146, 146, 145),
    border_focused: (17, 17, 17),
    border_thickness: 2.5,
    legend_off: (98, 97, 97),
    accent_default: (85, 85, 85),
    status_fg: (107, 78, 26),
    broadcast: (110, 45, 88),
    activity: (40, 72, 108),
    bell: (137, 58, 53),
    dim: (114, 114, 113),
    placeholder: (122, 122, 121),
    hint_fg: (112, 112, 112),
    find_hl_bg: (255, 221, 54),
    ansi: [
        (37, 37, 37),   // 0  black
        (146, 68, 61),  // 1  red
        (35, 104, 53),  // 2  green
        (118, 87, 0),   // 3  yellow
        (43, 93, 150),  // 4  blue
        (124, 73, 131), // 5  magenta
        (0, 102, 106),  // 6  cyan
        (73, 73, 73),   // 7  white
        (99, 99, 99),   // 8  bright black
        (127, 51, 45),  // 9  bright red
        (13, 87, 37),   // 10 bright green
        (98, 72, 0),    // 11 bright yellow
        (26, 76, 132),  // 12 bright blue
        (107, 57, 114), // 13 bright magenta
        (0, 84, 88),    // 14 bright cyan
        (39, 39, 38),   // 15 bright white
    ],
    dark: false,
    crt: None,
    modern: None,
    liquid: None,
    syntax: crate::SyntaxSlots::HOUSE,
    selection: None,
};

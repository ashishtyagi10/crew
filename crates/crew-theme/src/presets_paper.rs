//! Paper-family presets: ink on paper, dark and light.

use crate::{CrtStyle, ModernStyle, Theme};

/// High-contrast monochrome ("newspaper") dark theme — warm near-black/near-white
/// chrome for maximum legibility with minimal glare. The page leans warm
/// charcoal since the 2026-07-24 retune. Terminal ANSI output keeps
/// muted-but-readable colours so error/diff colour cues survive. The default.
///
/// See-through since 2026-10-09, like every mode: the page is a deep warm
/// smoke over the desktop ([`crate::presets_glass_clear::DARK_LIQUID`]),
/// with the glass's soft shadow behind the words and no paper of its own.
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
    bell: (254, 184, 174),
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
    grain: 0.0,
    crt: Some(CrtStyle {
        // Not a tube: no filament, so `is_crt` reads this as a page. The
        // bloom draws the focus ring's halo; the shade is the glass's text
        // shadow.
        scanline: 0.0,
        glow: 0.70,
        glow_radius: 13.0,
        flicker: 0.020,
        core: 0.0,
        etch: 0.0,
        shade: CrtStyle::GLASS_SHADE,
    }),
    modern: Some(ModernStyle {
        pole_a: (123, 184, 255),
        pole_b: (35, 199, 205),
        drift_ms: 6_000,
        // No wallpaper: the desktop is the background. The poles only light
        // the focus ring.
        dots: 0.0,
        wash: 0.0,
    }),
    liquid: Some(crate::presets_glass_clear::DARK_LIQUID),
};

/// The `light` mode's warm glass (2026-10-09: every mode see-through, and
/// light "a smoked clear glass with white words"): a warm, paper-toned smoke
/// over the desktop, cream-white ink and an amber accent — the paper page
/// as glass. Clear like `glass-clear` ([`crate::presets_glass_clear::LIGHT_LIQUID`]);
/// white words over a white desktop leave no room for a thinner smoke.
pub static PAPER_LIGHT: Theme = Theme {
    page_bg: (30, 26, 22),
    ink: (246, 242, 234),
    text_muted: (212, 208, 200),
    term_fg: (246, 242, 234),
    term_bg: (30, 26, 22),
    border_normal: (80, 75, 70),
    border_focused: (226, 232, 246),
    border_thickness: 3.0,
    legend_off: (155, 151, 145),
    accent_default: (255, 179, 102),
    status_fg: (255, 214, 10),
    broadcast: (210, 140, 255),
    activity: (100, 210, 255),
    bell: (255, 118, 110),
    dim: (136, 131, 125),
    placeholder: (127, 123, 117),
    hint_fg: (137, 133, 127),
    find_hl_bg: (46, 54, 121),
    ansi: [
        (107, 106, 104), // 0  black
        (255, 172, 161), // 1  red
        (129, 211, 145), // 2  green
        (231, 187, 93),  // 3  yellow
        (147, 197, 255), // 4  blue
        (237, 171, 244), // 5  magenta
        (59, 213, 219),  // 6  cyan
        (230, 228, 225), // 7  white
        (146, 144, 141), // 8  bright black
        (255, 203, 195), // 9  bright red
        (149, 232, 165), // 10 bright green
        (252, 207, 114), // 11 bright yellow
        (184, 217, 255), // 12 bright blue
        (250, 198, 255), // 13 bright magenta
        (87, 234, 240),  // 14 bright cyan
        (254, 252, 249), // 15 bright white
    ],
    dark: true,
    grain: 0.0,
    crt: Some(CrtStyle {
        // Not a tube: the bloom lights the focus ring, the shade is the
        // glass's text shadow.
        scanline: 0.0,
        glow: 0.5,
        glow_radius: 12.0,
        flicker: 0.015,
        core: 0.0,
        etch: 0.0,
        shade: CrtStyle::GLASS_SHADE,
    }),
    modern: Some(ModernStyle {
        pole_a: (255, 196, 120),
        pole_b: (240, 150, 120),
        drift_ms: 6_000,
        // No wallpaper: the desktop is the background.
        dots: 0.0,
        wash: 0.0,
    }),
    liquid: Some(crate::presets_glass_clear::LIGHT_LIQUID),
};

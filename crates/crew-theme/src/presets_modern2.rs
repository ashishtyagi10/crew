//! MODERN-family presets, the graphite half: graphene and cobalt — see
//! `presets_modern.rs` for the family charter (deep neutral pages, one
//! accent hue, clean bloom, gradient light-ring, zero retro knobs).

use crate::{CrtStyle, ModernStyle, Theme};

/// **Graphene** (the Codex look): the most restrained of the family — a pure
/// neutral near-black, quiet gray chrome, and a single mint-teal accent.
/// The glow is the family's tightest; the palette does almost nothing, on
/// purpose.
pub static GRAPHENE: Theme = Theme {
    page_bg: (14, 14, 16),
    ink: (235, 235, 238),
    text_muted: (197, 198, 200),
    term_fg: (236, 236, 240),
    term_bg: (14, 14, 16),
    border_normal: (68, 69, 71),
    border_focused: (94, 210, 180),
    border_thickness: 3.5,
    legend_off: (142, 142, 145),
    accent_default: (94, 210, 180),
    status_fg: (180, 220, 208),
    broadcast: (214, 160, 240),
    activity: (110, 212, 185),
    bell: (240, 200, 130),
    dim: (123, 124, 126),
    placeholder: (115, 116, 118),
    hint_fg: (125, 126, 128),
    find_hl_bg: (38, 66, 60),
    ansi: [
        (98, 99, 100),   // 0  black
        (255, 157, 145), // 1  red
        (119, 201, 135), // 2  green
        (221, 177, 82),  // 3  yellow
        (129, 187, 255), // 4  blue
        (226, 161, 234), // 5  magenta
        (41, 202, 209),  // 6  cyan
        (216, 217, 218), // 7  white
        (135, 136, 137), // 8  bright black
        (255, 189, 180), // 9  bright red
        (139, 222, 154), // 10 bright green
        (242, 197, 103), // 11 bright yellow
        (166, 207, 255), // 12 bright blue
        (247, 181, 255), // 13 bright magenta
        (73, 223, 230),  // 14 bright cyan
        (239, 240, 241), // 15 bright white
    ],
    dark: true,
    grain: 0.0,
    crt: Some(CrtStyle {
        curvature: 0.0,
        scanline: 0.0,
        glow: 0.6,
        glow_radius: 9.0,
        corner: 0.0,
        flicker: 0.02,
    }),
    modern: Some(ModernStyle {
        pole_a: (94, 210, 180),
        pole_b: (108, 210, 222),
        drift_ms: 7_000,
        dots: 0.20,
        wash: 0.15,
    }),
};

/// **Cobalt**: the electric one — a deep blue-black page with a blue→cyan
/// current running through the chrome. Copilot-adjacent, and the family's
/// coolest cast.
pub static COBALT: Theme = Theme {
    page_bg: (11, 15, 25),
    ink: (229, 237, 246),
    text_muted: (191, 199, 209),
    term_fg: (226, 233, 245),
    term_bg: (11, 15, 25),
    border_normal: (64, 70, 81),
    border_focused: (96, 165, 250),
    border_thickness: 3.5,
    legend_off: (137, 143, 156),
    accent_default: (103, 232, 249),
    status_fg: (170, 200, 248),
    broadcast: (196, 160, 252),
    activity: (110, 170, 250),
    bell: (250, 200, 140),
    dim: (118, 125, 137),
    placeholder: (110, 117, 129),
    hint_fg: (120, 126, 139),
    find_hl_bg: (38, 58, 100),
    ansi: [
        (97, 100, 105),  // 0  black
        (255, 158, 146), // 1  red
        (120, 202, 136), // 2  green
        (221, 178, 83),  // 3  yellow
        (130, 188, 255), // 4  blue
        (227, 161, 234), // 5  magenta
        (43, 203, 209),  // 6  cyan
        (215, 218, 224), // 7  white
        (134, 136, 142), // 8  bright black
        (255, 190, 181), // 9  bright red
        (140, 223, 155), // 10 bright green
        (242, 198, 104), // 11 bright yellow
        (168, 208, 255), // 12 bright blue
        (248, 181, 255), // 13 bright magenta
        (74, 224, 230),  // 14 bright cyan
        (238, 240, 246), // 15 bright white
    ],
    dark: true,
    grain: 0.0,
    crt: Some(CrtStyle {
        curvature: 0.0,
        scanline: 0.0,
        glow: 0.85,
        glow_radius: 12.0,
        corner: 0.0,
        flicker: 0.03,
    }),
    modern: Some(ModernStyle {
        pole_a: (96, 165, 250),
        pole_b: (103, 232, 249),
        drift_ms: 6_000,
        dots: 0.20,
        wash: 0.15,
    }),
};

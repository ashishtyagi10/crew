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
    ink: (236, 236, 240),
    text_muted: (200, 200, 208),
    term_fg: (236, 236, 240),
    term_bg: (14, 14, 16),
    border_normal: (54, 54, 60),
    border_focused: (94, 210, 180),
    border_thickness: 3.5,
    legend_off: (148, 148, 158),
    accent_default: (94, 210, 180),
    status_fg: (180, 220, 208),
    broadcast: (214, 160, 240),
    activity: (110, 212, 185),
    bell: (240, 200, 130),
    dim: (122, 122, 132),
    placeholder: (114, 114, 124),
    hint_fg: (120, 120, 130),
    find_hl_bg: (38, 66, 60),
    ansi: [
        (50, 50, 56),    // 0  black
        (244, 135, 135), // 1  red
        (110, 212, 170), // 2  green
        (235, 203, 139), // 3  yellow
        (124, 172, 248), // 4  blue
        (206, 148, 246), // 5  magenta
        (108, 210, 222), // 6  cyan
        (214, 214, 222), // 7  white
        (124, 124, 134), // 8  bright black
        (250, 160, 160), // 9  bright red
        (140, 224, 188), // 10 bright green
        (245, 218, 160), // 11 bright yellow
        (158, 192, 250), // 12 bright blue
        (220, 170, 250), // 13 bright magenta
        (140, 222, 232), // 14 bright cyan
        (240, 240, 246), // 15 bright white
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
    }),
};

/// **Cobalt**: the electric one — a deep blue-black page with a blue→cyan
/// current running through the chrome. Copilot-adjacent, and the family's
/// coolest cast.
pub static COBALT: Theme = Theme {
    page_bg: (11, 15, 25),
    ink: (226, 233, 245),
    text_muted: (190, 200, 218),
    term_fg: (226, 233, 245),
    term_bg: (11, 15, 25),
    border_normal: (50, 60, 86),
    border_focused: (96, 165, 250),
    border_thickness: 3.5,
    legend_off: (142, 154, 180),
    accent_default: (103, 232, 249),
    status_fg: (170, 200, 248),
    broadcast: (196, 160, 252),
    activity: (110, 170, 250),
    bell: (250, 200, 140),
    dim: (116, 128, 154),
    placeholder: (108, 120, 146),
    hint_fg: (114, 126, 152),
    find_hl_bg: (38, 58, 100),
    ansi: [
        (46, 54, 76),    // 0  black
        (248, 138, 148), // 1  red
        (120, 210, 158), // 2  green
        (248, 210, 116), // 3  yellow
        (110, 168, 254), // 4  blue
        (192, 150, 252), // 5  magenta
        (103, 216, 240), // 6  cyan
        (212, 220, 234), // 7  white
        (120, 132, 158), // 8  bright black
        (252, 162, 170), // 9  bright red
        (146, 222, 176), // 10 bright green
        (252, 224, 146), // 11 bright yellow
        (146, 190, 254), // 12 bright blue
        (210, 176, 253), // 13 bright magenta
        (135, 226, 245), // 14 bright cyan
        (236, 242, 250), // 15 bright white
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
    }),
};

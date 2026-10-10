//! **Glass clear**: the iPhone's liquid glass in its Clear look, what `glass`
//! serves (the user, 2026-10-09: "bright white would look great on glass").
//! Every pane is a slab of clear, lightly smoked glass over the desktop — the
//! wallpaper shows through in its own colours — and every word on it is
//! bright white. White reads there the way the iPhone makes it read: the
//! smoke dims the desktop a little, and a soft shadow behind each run of text
//! dims it a little more (`CrtStyle::GLASS_SHADE`), so the words hold over a
//! white desktop while the glass between them stays clear. The night glass
//! ([`crate::presets_glass`]) is its navy twin. The frosted light pair, sky
//! and dawn, folded into this one (the user, 2026-10-09: "should also be
//! clear"); their names still load it.
//!
//! Every derived role is what the ramp, the alarm and the wash produce for
//! this page and ink — the parity tests in `ramp_tests`, `signal_tests` and
//! `highlight_tests` are what put the numbers here.
use crate::{CrtStyle, LiquidStyle, ModernStyle, Theme};

pub static GLASS_CLEAR: Theme = Theme {
    page_bg: (17, 18, 22),
    ink: (238, 239, 241),
    text_muted: (199, 201, 204),
    term_fg: (238, 239, 241),
    term_bg: (17, 18, 22),
    border_normal: (69, 71, 75),
    border_focused: (226, 232, 246),
    border_thickness: 2.0,
    legend_off: (143, 145, 149),
    accent_default: (255, 125, 165),
    status_fg: (255, 214, 10),
    broadcast: (191, 90, 242),
    activity: (100, 210, 255),
    bell: (255, 118, 110),
    dim: (124, 126, 130),
    placeholder: (116, 118, 122),
    hint_fg: (126, 128, 132),
    find_hl_bg: (44, 52, 112),
    ansi: [
        (100, 101, 103), // 0  black
        (255, 161, 149), // 1  red
        (122, 204, 138), // 2  green
        (223, 180, 85),  // 3  yellow
        (134, 190, 255), // 4  blue
        (229, 163, 236), // 5  magenta
        (46, 205, 211),  // 6  cyan
        (219, 220, 222), // 7  white
        (137, 138, 140), // 8  bright black
        (255, 193, 184), // 9  bright red
        (142, 225, 158), // 10 bright green
        (244, 200, 106), // 11 bright yellow
        (171, 210, 255), // 12 bright blue
        (248, 185, 255), // 13 bright magenta
        (77, 226, 232),  // 14 bright cyan
        (242, 243, 245), // 15 bright white
    ],
    dark: true,
    crt: Some(CLEAR_BLOOM),
    modern: Some(ModernStyle {
        pole_a: (70, 130, 255),
        pole_b: (255, 64, 150),
        drift_ms: 6_000,
    }),
    liquid: Some(CLEAR_LIQUID),
};

/// The `dark` mode's glass (the user, 2026-10-09: dark, light and CRT "with
/// the same pattern as glass"): the clear glass's optics with a deeper
/// smoke. The window keeps more of the desktop out and each pane more of
/// what is left, so a pane hides 0.71 of the desktop where the clear glass
/// hides 0.59 — still glass, the desktop's shapes show through, but a dark
/// room rather than a clear one.
pub(crate) const DARK_LIQUID: LiquidStyle = LiquidStyle {
    window: 0.35,
    body: 0.55,
    ..CLEAR_LIQUID
};

/// The `light` mode's glass: as clear as `glass-clear`, a touch denser in
/// the body. Light was meant to be the thinner smoke, but white words over a
/// white desktop need a pane to hide about 0.6 of it (with the text shadow)
/// to hold 4.5:1 — the clear glass is already at that edge — so light is
/// told apart by its tint (warm, rose) rather than by its thickness.
pub(crate) const LIGHT_LIQUID: LiquidStyle = LiquidStyle {
    body: 0.47,
    ..CLEAR_LIQUID
};

/// The white-text glass's tube settings, shared with the night glass: a
/// little glow, no tube, and the soft shadow white words need over a clear
/// pane.
pub(crate) const CLEAR_BLOOM: CrtStyle = CrtStyle {
    scanline: 0.0,
    glow: 0.45,
    glow_radius: 12.0,
    flicker: 0.0,
    core: 0.0,
    etch: 0.0,
    shade: CrtStyle::GLASS_SHADE,
};

/// Clear glass, shared with the night glass: a pane hides 0.59 of the
/// desktop, where the old night glass hid 0.66 — white text holds over a
/// white desktop because of the shadow behind it, not because the smoke is
/// thick.
pub(crate) const CLEAR_LIQUID: LiquidStyle = LiquidStyle {
    // No lens: the glass paints no wallpaper, so bending, blurring and
    // splitting the page drew nothing — eleven texture reads a pixel for a
    // solid colour. The desktop's blur is the window server's.
    refract: 0.0,
    bevel: 16.0,
    blur: 0.0,
    dispersion: 0.0,
    clear_rim: 0.75,
    vibrance: 1.35,
    window: 0.25,
    body: 0.45,
    desktop_blur: 20.0,
};

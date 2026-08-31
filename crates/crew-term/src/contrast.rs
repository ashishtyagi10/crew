//! Minimum-contrast floor for program-painted text (à la iTerm2's "Minimum
//! Contrast"). Agent CLIs sample the background once at startup — after a live
//! theme switch (or a wrong guess) they keep painting truecolor tuned to the
//! opposite background, which lands as white-on-white / black-on-black. The
//! terminal is the only place that always knows both colours, so `cells()`
//! nudges any foreground too close to its background just far enough to read,
//! preserving hue.
use std::sync::OnceLock;

/// Minimum WCAG-style contrast ratio enforced between a cell's fg and bg.
pub(crate) const MIN_CONTRAST: f32 = 3.0;

/// The floor for text the program painted a background BEHIND.
///
/// [`MIN_CONTRAST`] is a *rescue*: it exists for a program that guessed the
/// page wrong and is painting for the other theme. A cell whose background
/// the program also set is not a guess — it is a TUI's selected row, a status
/// bar, a diff block — and it is the thing on screen most meant to be read.
///
/// The two floors have to differ because the ansi table's own contract works
/// against this one. `black` is LIFTED so `\x1b[30m` reads on the page, which
/// hands a mid-grey to a cell whose background is a light green: a file
/// picker's selected row (`\x1b[42;30m`) came out at **2.98:1** while the
/// plain rows around it sat at 9.65, so the row the program was pointing at
/// was the least legible line on the screen.
pub(crate) const PAINTED_CONTRAST: f32 = 4.5;

/// sRGB byte → linear-light, via a table (this runs for every rendered cell).
fn to_linear(c: u8) -> f32 {
    static LUT: OnceLock<[f32; 256]> = OnceLock::new();
    LUT.get_or_init(|| {
        let mut t = [0.0f32; 256];
        for (i, v) in t.iter_mut().enumerate() {
            let c = i as f32 / 255.0;
            *v = if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            };
        }
        t
    })[c as usize]
}

/// Linear-light → sRGB byte (only runs for the rare cell that needs fixing).
fn to_srgb(l: f32) -> u8 {
    let l = l.clamp(0.0, 1.0);
    let c = if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    (c * 255.0).round() as u8
}

/// BT.709 relative luminance (0.0 black … 1.0 white) — the same formula the
/// agent CLIs use to classify a background as light or dark.
pub(crate) fn luminance((r, g, b): (u8, u8, u8)) -> f32 {
    0.2126 * to_linear(r) + 0.7152 * to_linear(g) + 0.0722 * to_linear(b)
}

/// WCAG contrast ratio between two colours (1.0 … 21.0).
pub(crate) fn ratio(a: (u8, u8, u8), b: (u8, u8, u8)) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Enforce [`MIN_CONTRAST`] between `fg` and `bg`: a foreground too close to
/// its background is darkened (light background) or lightened (dark
/// background) just enough to read. Channels scale together in linear light,
/// so hue survives; already-readable colours pass through untouched.
pub(crate) fn ensure_min_contrast(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> (u8, u8, u8) {
    ensure_contrast(fg, bg, MIN_CONTRAST)
}

/// The floor for `fg` on `bg`, chosen by whether the program painted that
/// background itself (see [`PAINTED_CONTRAST`]).
pub(crate) fn ensure_readable(fg: (u8, u8, u8), bg: (u8, u8, u8), painted: bool) -> (u8, u8, u8) {
    let min = if painted {
        PAINTED_CONTRAST
    } else {
        MIN_CONTRAST
    };
    ensure_contrast(fg, bg, min)
}

/// How far SGR 2 moves a foreground toward its background, in linear light.
/// Enough to read as a second voice; not so far that the line stops being a
/// line.
const DIM_MIX: f32 = 0.45;

/// The floor a *dim* cell is held to. Lower than [`MIN_CONTRAST`] on purpose —
/// the program asked for quieter, and answering with the same contrast as
/// body text would be ignoring it — but a dim that cannot be read is a line
/// dropped rather than a line whispered.
pub(crate) const DIM_CONTRAST: f32 = 2.0;

/// SGR 2: the same colour, spoken quietly. Mixed toward the background in
/// linear light so hue survives, then floored — agent CLIs put half their
/// output in dim, and on a page whose colours they guessed wrong that mix can
/// land on top of the background.
pub(crate) fn dimmed(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> (u8, u8, u8) {
    let mix = |f: u8, b: u8| {
        let (f, b) = (to_linear(f), to_linear(b));
        to_srgb(f + (b - f) * DIM_MIX)
    };
    let out = (mix(fg.0, bg.0), mix(fg.1, bg.1), mix(fg.2, bg.2));
    ensure_contrast(out, bg, DIM_CONTRAST)
}

/// [`ensure_min_contrast`] against an explicit floor.
fn ensure_contrast(fg: (u8, u8, u8), bg: (u8, u8, u8), min: f32) -> (u8, u8, u8) {
    if ratio(fg, bg) >= min {
        return fg;
    }
    let lf = luminance(fg);
    let lb = luminance(bg);
    let lin = (to_linear(fg.0), to_linear(fg.1), to_linear(fg.2));
    if lb >= 0.18 {
        // Light-ish background → darken the foreground to the target luminance.
        let target = ((lb + 0.05) / min - 0.05).max(0.0);
        let k = if lf > 0.0 { target / lf } else { 0.0 };
        (to_srgb(lin.0 * k), to_srgb(lin.1 * k), to_srgb(lin.2 * k))
    } else {
        // Dark background → lighten the foreground toward white.
        let target = (min * (lb + 0.05) - 0.05).min(1.0);
        let t = if lf < 1.0 {
            (target - lf) / (1.0 - lf)
        } else {
            0.0
        };
        let up = |c: f32| to_srgb(c + t * (1.0 - c));
        (up(lin.0), up(lin.1), up(lin.2))
    }
}

/// The `COLORFGBG` value (`"fg;bg"` in ANSI indices) matching a terminal
/// background — the env-var fallback agent CLIs read when OSC 11 goes
/// unanswered. Light page → dark-on-light (`0;15`); dark page → `15;0`.
pub(crate) fn colorfgbg_for(term_bg: (u8, u8, u8)) -> &'static str {
    if luminance(term_bg) > 0.5 {
        "0;15"
    } else {
        "15;0"
    }
}

#[cfg(test)]
#[path = "contrast_tests.rs"]
mod tests;

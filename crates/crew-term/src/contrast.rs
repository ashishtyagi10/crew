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
    if ratio(fg, bg) >= MIN_CONTRAST {
        return fg;
    }
    let lf = luminance(fg);
    let lb = luminance(bg);
    let lin = (to_linear(fg.0), to_linear(fg.1), to_linear(fg.2));
    if lb >= 0.18 {
        // Light-ish background → darken the foreground to the target luminance.
        let target = ((lb + 0.05) / MIN_CONTRAST - 0.05).max(0.0);
        let k = if lf > 0.0 { target / lf } else { 0.0 };
        (to_srgb(lin.0 * k), to_srgb(lin.1 * k), to_srgb(lin.2 * k))
    } else {
        // Dark background → lighten the foreground toward white.
        let target = (MIN_CONTRAST * (lb + 0.05) - 0.05).min(1.0);
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
mod tests {
    use super::*;

    #[test]
    fn readable_colours_pass_through_unchanged() {
        // Ink on paper and phosphor green on near-black both clear the floor.
        assert_eq!(
            ensure_min_contrast((22, 20, 18), (246, 243, 236)),
            (22, 20, 18)
        );
        assert_eq!(
            ensure_min_contrast((0, 255, 102), (3, 10, 5)),
            (0, 255, 102)
        );
    }

    #[test]
    fn white_on_white_is_darkened_to_the_floor() {
        let bg = (246, 243, 236); // paper-light term_bg
        for fg in [(255, 255, 255), (235, 235, 235), (246, 243, 236)] {
            let fixed = ensure_min_contrast(fg, bg);
            assert!(
                ratio(fixed, bg) >= MIN_CONTRAST - 0.1,
                "{fg:?} → {fixed:?} ratio {}",
                ratio(fixed, bg)
            );
        }
    }

    #[test]
    fn black_on_black_is_lightened_to_the_floor() {
        let bg = (8, 8, 8); // paper-dark term_bg
        for fg in [(0, 0, 0), (30, 30, 30), (8, 8, 8)] {
            let fixed = ensure_min_contrast(fg, bg);
            assert!(
                ratio(fixed, bg) >= MIN_CONTRAST - 0.1,
                "{fg:?} → {fixed:?} ratio {}",
                ratio(fixed, bg)
            );
        }
    }

    #[test]
    fn hue_survives_the_nudge() {
        // A washed-out warm yellow on paper stays warm after darkening.
        let fixed = ensure_min_contrast((240, 220, 160), (246, 243, 236));
        assert!(
            fixed.0 > fixed.2,
            "red channel should stay dominant: {fixed:?}"
        );
        assert!(ratio(fixed, (246, 243, 236)) >= MIN_CONTRAST - 0.1);
    }

    #[test]
    fn colorfgbg_matches_background_lightness() {
        assert_eq!(colorfgbg_for((246, 243, 236)), "0;15");
        assert_eq!(colorfgbg_for((8, 8, 8)), "15;0");
        assert_eq!(colorfgbg_for((3, 10, 5)), "15;0");
    }
}

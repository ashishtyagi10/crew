use super::*;
use crate::glassborder::lift;
use crate::{ThemeId, ALL_THEMES};

/// The served (lifted) quiet legend reads over a black desktop and a white
/// one on every glass palette. On main the light glass's `legend_off`
/// measured 2.0–2.3 on its own ground over black (the user's screenshot,
/// 2026-10-09: `#585C66` on lavender `#B4BCD4`).
#[test]
fn quiet_legends_read_over_any_desktop() {
    for id in ALL_THEMES {
        let t = lift(id.theme());
        let g = grounds(&t);
        if g.is_empty() {
            continue;
        }
        let got = worst(t.legend_off, &g);
        assert!(got >= LABEL_FLOOR, "{}: legend_off {got:.2}", id.as_str());
    }
}

/// A pane's own hue — mid-tone, chosen to read on the PAGE — is walked to a
/// legend that reads on the glass, at the focused floor too, and keeps its
/// hue: it still says which pane it is.
#[test]
fn any_hue_becomes_a_legend_on_glass() {
    let hues = [
        (55, 82, 113),
        (200, 90, 120),
        (40, 170, 120),
        (220, 160, 40),
    ];
    for id in [ThemeId::GlassClear, ThemeId::GlassNight] {
        let t = id.theme();
        let g = grounds(t);
        for hue in hues {
            let got = legible(t, hue, FOCUSED_LABEL_FLOOR);
            assert!(
                worst(got, &g) >= FOCUSED_LABEL_FLOOR,
                "{}: {hue:?} → {got:?} {:.2}",
                id.as_str(),
                worst(got, &g)
            );
            let (a, b) = (oklch::from_srgb(hue), oklch::from_srgb(got));
            let turn = ((a.h - b.h).abs() % 360.0).min(360.0 - (a.h - b.h).abs() % 360.0);
            assert!(
                turn < 15.0 || b.c < 0.03,
                "{}: {hue:?} → {got:?} turned {turn:.0}°",
                id.as_str()
            );
        }
    }
}

/// The glass's labels go light — white words on smoked glass, the iPhone's
/// Clear look — whichever the colour started as.
#[test]
fn labels_take_the_glass_s_pole() {
    let mid = (128, 128, 128);
    for id in [ThemeId::GlassClear, ThemeId::GlassNight] {
        let got = legible(id.theme(), mid, LABEL_FLOOR);
        assert!(crate::relative_luminance(got) > crate::relative_luminance(mid));
    }
}

/// Nothing changes off glass: every other palette's legends are its own.
#[test]
fn other_palettes_keep_their_legends() {
    for id in ALL_THEMES {
        let t = id.theme();
        if t.liquid.is_some() {
            continue;
        }
        assert!(grounds(t).is_empty(), "{}", id.as_str());
        assert_eq!(legible(t, (128, 128, 128), 7.0), (128, 128, 128));
    }
}

/// The veil thickens the glass behind a legend: it hides more of the
/// desktop than the pane's body does, and less than all of it — still glass.
#[test]
fn the_veil_is_thicker_glass_not_a_tab() {
    for id in ALL_THEMES {
        let Some(l) = id.theme().liquid else { continue };
        let shade = shade(id.theme());
        let veil = legend_cover(l, shade);
        assert!(veil > l.text_cover(shade), "{}", id.as_str());
        assert!(veil < 0.9, "{}: veil hides {veil:.2}", id.as_str());
    }
}

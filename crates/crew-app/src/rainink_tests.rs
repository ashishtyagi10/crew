use super::*;
use crew_theme::oklch::from_srgb;
use crew_theme::{contrast_ratio, ALL_THEMES};

/// Hue distance in degrees, the short way round.
fn hue_gap(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// On every glass, classic and soft palette the rain is the accent's hue,
/// and visibly a colour rather than a grey.
#[test]
fn the_rain_wears_the_accent_s_hue_on_every_colour_palette() {
    let mut seen = 0;
    for id in ALL_THEMES {
        let t = id.theme();
        if t.is_print() || t.is_tube() {
            continue;
        }
        seen += 1;
        let (r, a) = (
            from_srgb(rain_ink(t, t.accent_default)),
            from_srgb(t.accent_default),
        );
        assert!(r.c >= 0.04, "{}: rain chroma {}", id.as_str(), r.c);
        assert!(
            hue_gap(r.h, a.h) < 12.0,
            "{}: rain hue {} vs accent {}",
            id.as_str(),
            r.h,
            a.h
        );
    }
    assert_eq!(seen, 16, "the colour palettes");
}

/// The tint costs the rain none of its quiet: it reads against the page
/// about as `text_muted` does, and never as loud as the ink.
#[test]
fn the_rain_is_as_quiet_as_muted_text_on_every_palette() {
    for id in ALL_THEMES {
        let t = id.theme();
        let rain = contrast_ratio(rain_ink(t, t.accent_default), t.page_bg);
        let muted = contrast_ratio(t.text_muted, t.page_bg);
        let ink = contrast_ratio(t.ink, t.page_bg);
        assert!(
            (rain / muted - 1.0).abs() < 0.25,
            "{}: rain {rain} vs muted {muted}",
            id.as_str()
        );
        assert!(rain < ink, "{}: rain {rain} under ink {ink}", id.as_str());
    }
}

/// Print has one colour and a tube has its phosphor: both keep the muted ink.
#[test]
fn ink_pages_and_tubes_keep_their_own_rain() {
    for id in ALL_THEMES {
        let t = id.theme();
        if t.is_print() || t.is_tube() {
            assert_eq!(rain_ink(t, (255, 0, 128)), t.text_muted, "{}", id.as_str());
        }
    }
}

/// The rain follows an accent the user set, not only the palette's own.
#[test]
fn a_user_accent_moves_the_rain() {
    let t = crew_theme::ThemeId::Nebula.theme();
    let (red, blue) = (rain_ink(t, (255, 60, 60)), rain_ink(t, (60, 120, 255)));
    assert!(hue_gap(from_srgb(red).h, from_srgb(blue).h) > 90.0);
}

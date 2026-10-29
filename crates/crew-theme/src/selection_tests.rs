//! A classic selects in its scheme's own tint (2026-10-29). Selecting in the
//! accent's hue served crew's own palettes, whose accent is their identity,
//! but a classic's accent is one signal among its scheme's colours:
//! solarized's yellow accent made its selection a mud olive (92, 73, 1)
//! where solarized selects in deep teal, and gruvbox's orange made a brown
//! where gruvbox selects in warm grey.
use super::*;
use crate::ALL_THEMES;

/// The tint's hue survives, and a grey scheme selection stays a grey rather
/// than picking up a hue rounding happened to give it.
#[test]
fn a_classic_selects_in_its_schemes_tint() {
    let gap = |a: f32, b: f32| (a - b).rem_euclid(360.0).min((b - a).rem_euclid(360.0));
    let mut classics = 0;
    for id in ALL_THEMES {
        let t = id.theme();
        let Some(own) = t.selection else { continue };
        classics += 1;
        let (got, want) = (oklch::from_srgb(selection_bg(t)), oklch::from_srgb(own));
        assert!(
            (got.c - want.c).abs() < 0.02,
            "{}: chroma {:.3}, the scheme's {:.3}",
            id.as_str(),
            got.c,
            want.c
        );
        if want.c >= 0.03 {
            let d = gap(got.h, want.h);
            assert!(d < 15.0, "{}: hue {d:.0}° off the scheme's", id.as_str());
        }
    }
    assert_eq!(classics, 5);
}

/// Only the classics name a selection; crew's own palettes select in their
/// accent, as they always have.
#[test]
fn only_a_classic_names_its_selection() {
    for id in ALL_THEMES {
        let t = id.theme();
        let classic = id.mode() == crate::RandomMode::Classic;
        assert_eq!(t.selection.is_some(), classic, "{}", id.as_str());
    }
}

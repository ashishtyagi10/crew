use super::*;
use crate::{ThemeId, ALL_THEMES};

/// The shift and the theme selection are both process-wide, so every test
/// below that writes one takes the crate's lock — otherwise a parallel test
/// reading `poles()` sees another test's theme, or its offset.
use crate::test_guard as guard;

/// Every theme's poles, for the sweeps below.
fn all_poles() -> Vec<(u8, u8, u8)> {
    ALL_THEMES
        .iter()
        .filter_map(|t| t.theme().modern)
        .flat_map(|m| [m.pole_a, m.pole_b])
        .collect()
}

/// The determinism contract: zero offset is not "almost the same colour", it
/// is the same bytes. Every static-frame test in the workspace rests on this.
#[test]
fn a_zero_shift_is_the_identity() {
    for p in all_poles() {
        assert_eq!(shifted(p, 0.0), p, "{p:?}");
    }
}

/// A bad number upstream must freeze the colour, never corrupt it.
#[test]
fn a_non_finite_shift_is_the_identity() {
    for p in all_poles() {
        assert_eq!(shifted(p, f32::NAN), p, "{p:?}");
        assert_eq!(shifted(p, f32::INFINITY), p, "{p:?}");
    }
}

/// The safety argument, measured: rotating hue must not move OKLCH lightness,
/// because every contrast guarantee in the palette suite is a function of it.
#[test]
fn rotation_holds_oklch_lightness() {
    for p in all_poles() {
        let l0 = oklch::from_srgb(p).l;
        for deg in [-90.0, -45.0, -16.0, 16.0, 45.0, 90.0_f32] {
            let l1 = oklch::from_srgb(shifted(p, deg)).l;
            assert!(
                (l1 - l0).abs() < 0.02,
                "{p:?} at {deg}°: L {l0:.3} -> {l1:.3}"
            );
        }
    }
}

/// The consequence of the above, measured the way the palettes are — but
/// measured honestly: WCAG contrast is Rec.709 relative luminance, which is
/// NOT the perceptual lightness the rotation holds, so a pure hue turn does
/// move a ratio a little. The question is by how much, and the answer has to
/// be small: the wash has only 4-16% headroom over the washed page
/// (v0.18.26), so a rotation that moved a ratio by a tenth would be spending
/// headroom that is not there.
///
/// Bound is the LADDER's real range (±38°, `lively`), not the ±90° clamp.
/// Measured worst across all eight themes at the time of writing: Blossom's
/// `pole_a` at -38°, 7.1%. The clamp's own range is covered by the floor
/// test below, which is the one that actually protects the reader.
#[test]
fn rotation_holds_contrast_across_the_ladder() {
    for id in ALL_THEMES.iter() {
        let t = id.theme();
        let Some(m) = t.modern else { continue };
        for p in [m.pole_a, m.pole_b] {
            let base = crate::contrast_ratio(p, t.page_bg);
            for deg in [-38.0, -16.0, 16.0, 38.0_f32] {
                let now = crate::contrast_ratio(shifted(p, deg), t.page_bg);
                assert!(
                    (now - base).abs() / base < 0.08,
                    "{id:?} {p:?} at {deg}°: {base:.2} -> {now:.2}"
                );
            }
        }
    }
}

/// The guarantee that matters at any offset a hand-edited config can reach:
/// a pole stays a stroke you can see. 3.0 is the WCAG non-text floor, which
/// is what these colours are measured against — they are borders, dots and
/// pools of light, not body text (`readable` owns the 4.5 roles, and none of
/// them go through this module). Swept a degree at a time across the whole
/// clamp, so no narrow hue notch can hide between samples. Measured minimum
/// at the time of writing: 3.94, Blossom's `pole_b`.
#[test]
fn no_offset_can_take_a_pole_below_the_stroke_floor() {
    for id in ALL_THEMES.iter() {
        let t = id.theme();
        let Some(m) = t.modern else { continue };
        for p in [m.pole_a, m.pole_b] {
            for step in -90..=90 {
                let now = crate::contrast_ratio(shifted(p, step as f32), t.page_bg);
                assert!(now >= 3.0, "{id:?} {p:?} at {step}°: {now:.2}");
            }
        }
    }
}

/// A grey has no hue to turn. Without the chroma guard the OKLCH round trip
/// would still nudge the bytes, which would show up as a colour cast on the
/// one thing that must not have one.
#[test]
fn a_neutral_grey_never_moves() {
    for g in [(0, 0, 0), (17, 17, 17), (128, 128, 128), (255, 255, 255)] {
        assert_eq!(shifted(g, 45.0), g, "{g:?}");
    }
}

/// A rotation must actually be visible — the point of the feature. 16° is the
/// `subtle` rung, so it is the floor the ladder has to clear.
#[test]
fn a_subtle_rotation_is_a_visible_move() {
    for p in all_poles() {
        if oklch::from_srgb(p).c <= 0.02 {
            continue; // near-grey pole: nothing to see, by construction
        }
        let moved = shifted(p, 16.0);
        assert!(
            oklch::distance(p, moved) > 0.005,
            "{p:?} -> {moved:?} is not a visible move"
        );
    }
}

/// Both poles turn the same way by the same amount: the interval between them
/// is the theme's signature.
#[test]
fn both_poles_turn_together() {
    let _g = guard();
    let prev = shift();
    crate::apply_selection(crate::Selection::Fixed(ThemeId::Nebula), 0);
    let m = crate::theme().modern.expect("modern theme has poles");
    let spread = |a: (u8, u8, u8), b: (u8, u8, u8)| {
        (oklch::from_srgb(b).h - oklch::from_srgb(a).h).rem_euclid(360.0)
    };
    let before = spread(m.pole_a, m.pole_b);
    set_shift(30.0);
    let (a, b) = poles().expect("modern theme has poles");
    assert_eq!(a, shifted(m.pole_a, 30.0));
    assert_eq!(b, shifted(m.pole_b, 30.0));
    assert!(
        (spread(a, b) - before).abs() < 6.0,
        "spread {before:.1}° -> {:.1}°",
        spread(a, b)
    );
    set_shift(prev);
}

/// The clamp, and the round trip through the global.
#[test]
fn the_global_clamps_and_round_trips() {
    let _g = guard();
    let prev = shift();
    for d in [0.0, -12.5, 16.0, 90.0_f32] {
        set_shift(d);
        assert_eq!(shift(), d);
    }
    set_shift(400.0);
    assert_eq!(shift(), MAX_SHIFT_DEG);
    set_shift(-400.0);
    assert_eq!(shift(), -MAX_SHIFT_DEG);
    set_shift(f32::NAN);
    assert_eq!(shift(), 0.0);
    set_shift(prev);
}

/// At rest `poles()` is the theme's own constants — the fallback path every
/// caller had before this module existed.
#[test]
fn at_rest_poles_are_the_themes_own() {
    let _g = guard();
    let prev = shift();
    set_shift(0.0);
    for id in ALL_THEMES.iter() {
        crate::apply_selection(crate::Selection::Fixed(*id), 0);
        let m = crate::theme().modern.expect("every theme ships poles");
        assert_eq!(poles(), Some((m.pole_a, m.pole_b)), "{id:?}");
    }
    set_shift(prev);
}

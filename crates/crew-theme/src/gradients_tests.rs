use super::*;
use crate::{oklch, poleshift, ALL_THEMES};

/// Every name on the shelf resolves, and resolves to the pair it names.
#[test]
fn every_name_resolves() {
    for g in GRADIENTS {
        assert_eq!(by_name(g.name), Some(g.poles), "{}", g.name);
        assert_eq!(by_name(&g.name.to_uppercase()), Some(g.poles));
        assert_eq!(by_name(&format!("  {}  ", g.name)), Some(g.poles));
    }
    assert_eq!(by_name("chartreuse"), None);
    assert_eq!(by_name(""), None);
}

/// Names are unique and typeable: a duplicate would make one entry
/// unreachable, and a name with a space in it could never be parsed apart
/// from a two-colour argument.
#[test]
fn names_are_unique_and_typeable() {
    let mut seen = Vec::new();
    for g in GRADIENTS {
        assert!(!seen.contains(&g.name), "duplicate name {}", g.name);
        seen.push(g.name);
        assert!(
            g.name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
            "{} must be lowercase ascii to be typeable",
            g.name
        );
        assert!(!g.about.is_empty(), "{} needs a description", g.name);
    }
}

/// A pair whose two poles have no interval between them is not a gradient —
/// `mono` is the one deliberate exception, and it says so in its own name.
#[test]
fn every_pair_but_mono_actually_travels() {
    for g in GRADIENTS {
        let (a, b) = g.poles;
        let spread = (oklch::from_srgb(b).h - oklch::from_srgb(a).h)
            .abs()
            .min(360.0 - (oklch::from_srgb(b).h - oklch::from_srgb(a).h).abs());
        if g.name == "mono" {
            assert_eq!(a, b, "mono must be one colour twice");
            continue;
        }
        assert!(spread > 20.0, "{} spans only {spread:.0}°", g.name);
    }
}

/// Two presets that land on the same pair of hues are one preset with two
/// names — the shelf is short precisely so every entry is a different answer.
#[test]
fn no_two_presets_are_the_same_gradient() {
    let hue = |c: (u8, u8, u8)| oklch::from_srgb(c).h;
    for (i, g) in GRADIENTS.iter().enumerate() {
        if g.name == "mono" {
            continue; // grey has no hue to compare
        }
        for h in &GRADIENTS[i + 1..] {
            if h.name == "mono" {
                continue;
            }
            let da = (hue(g.poles.0) - hue(h.poles.0)).abs();
            let db = (hue(g.poles.1) - hue(h.poles.1)).abs();
            assert!(
                da > 12.0 || db > 12.0,
                "{} and {} are the same gradient ({da:.0}°, {db:.0}°)",
                g.name,
                h.name
            );
        }
    }
}

/// The guarantee that makes a shelf of colours safe to ship: put any of them
/// on any palette and the poles still clear the WCAG non-text floor, because
/// the lightness is never theirs to set.
#[test]
fn every_preset_is_readable_on_every_palette() {
    let _g = crate::test_guard();
    let (prev_c, prev_s) = (poleshift::custom(), poleshift::shift());
    poleshift::set_shift(0.0);
    for g in GRADIENTS {
        poleshift::set_custom(Some(g.poles));
        for id in ALL_THEMES.iter() {
            crate::apply_selection(crate::Selection::Fixed(*id), 0);
            let page = crate::theme().page_bg;
            let (a, b) = poleshift::poles().expect("every theme ships poles");
            for p in [a, b] {
                let r = crate::contrast_ratio(p, page);
                assert!(r >= 3.0, "{} on {id:?}: {r:.2}", g.name);
            }
        }
    }
    poleshift::set_custom(prev_c);
    poleshift::set_shift(prev_s);
}

/// A preset keeps its hue when it lands — the re-lighting must not quietly
/// turn `ember` into the theme's own violet.
#[test]
fn a_preset_keeps_its_hue_on_every_palette() {
    let _g = crate::test_guard();
    let (prev_c, prev_s) = (poleshift::custom(), poleshift::shift());
    poleshift::set_shift(0.0);
    for g in GRADIENTS {
        if g.name == "mono" {
            continue; // grey has no hue to keep
        }
        poleshift::set_custom(Some(g.poles));
        for id in ALL_THEMES.iter() {
            crate::apply_selection(crate::Selection::Fixed(*id), 0);
            let (a, _) = poleshift::poles().expect("every theme ships poles");
            let (want, got) = (oklch::from_srgb(g.poles.0).h, oklch::from_srgb(a).h);
            let d = (want - got).abs().min(360.0 - (want - got).abs());
            assert!(d < 6.0, "{} on {id:?}: {want:.0}° -> {got:.0}°", g.name);
        }
    }
    poleshift::set_custom(prev_c);
    poleshift::set_shift(prev_s);
}

/// The reverse lookup, so `/gradient` can report a name rather than two hex
/// codes for a gradient picked by name.
#[test]
fn a_pair_is_recognised_as_its_name() {
    for g in GRADIENTS {
        assert_eq!(name_of(g.poles), Some(g.name));
    }
    assert_eq!(name_of(((1, 2, 3), (4, 5, 6))), None);
}

/// The step key's walk: out of the theme's own gradient, along all eight, and
/// back out to the theme's own. A cycle that could not come home would be a
/// one-way door.
#[test]
fn the_walk_visits_every_preset_and_returns_home() {
    let mut at = None;
    let mut seen = Vec::new();
    for _ in 0..GRADIENTS.len() {
        at = next(at);
        seen.push(at.expect("the walk must not end early"));
    }
    assert_eq!(
        seen,
        GRADIENTS.iter().map(|g| g.poles).collect::<Vec<_>>(),
        "the walk must be the shelf, in order"
    );
    assert_eq!(next(at), None, "the lap must end at the theme's own");
    assert_eq!(next(None), GRADIENTS.first().map(|g| g.poles));
}

/// A gradient of someone's own is not on the shelf, so the step key has to
/// decide where it lands rather than stranding them — it enters at the start.
#[test]
fn a_pair_off_the_shelf_joins_the_walk_at_the_start() {
    let mine = Some(((1, 2, 3), (4, 5, 6)));
    assert_eq!(next(mine), GRADIENTS.first().map(|g| g.poles));
}

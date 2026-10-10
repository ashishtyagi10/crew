use super::*;
use crate::glassborder::lift;
use crate::glasslegend::worst;
use crate::{relative_luminance, ThemeId, ALL_THEMES};

/// Every text role of every glass palette, as served on a sheer window,
/// reads over a black desktop and a white one at its floor. On main, light
/// glass over black measured dim 1.46, the accent 1.62, an error 1.86.
#[test]
fn every_role_reads_over_any_desktop() {
    let mut under = Vec::new();
    for id in ALL_THEMES {
        let base = id.theme();
        let g = pane_grounds(base);
        if g.is_empty() {
            continue;
        }
        let t = lift(base);
        let mut roles = vec![
            ("ink", t.ink, TEXT_FLOOR),
            ("term_fg", t.term_fg, TEXT_FLOOR),
            ("status_fg", t.status_fg, TEXT_FLOOR),
            ("bell", t.bell, SECONDARY_FLOOR),
            ("text_muted", t.text_muted, SECONDARY_FLOOR),
            ("dim", t.dim, MARK_FLOOR),
            ("placeholder", t.placeholder, MARK_FLOOR),
            ("hint_fg", t.hint_fg, MARK_FLOOR),
        ];
        roles.extend(t.ansi.iter().skip(1).map(|&c| ("ansi", c, SECONDARY_FLOOR)));
        for (role, c, floor) in roles {
            let got = worst(c, &g);
            // A hue can top out a hair short of the floor; no more than that.
            if got < floor - 0.05 {
                under.push(format!("{} {role} {c:?} {got:.2}", id.as_str()));
            }
        }
    }
    assert!(
        under.is_empty(),
        "under the floor:\n  {}",
        under.join("\n  ")
    );
}

/// The ladder survives: body text is still the strongest, secondary text
/// next, hints after it — flooring moved them, it did not flatten them.
#[test]
fn the_ladder_survives_the_floor() {
    for id in [ThemeId::GlassClear, ThemeId::GlassNight] {
        let t = lift(id.theme());
        let g = pane_grounds(id.theme());
        let (ink, muted, dim) = (worst(t.ink, &g), worst(t.text_muted, &g), worst(t.dim, &g));
        assert!(
            ink > muted && muted > dim,
            "{}: {ink:.2} {muted:.2} {dim:.2}",
            id.as_str()
        );
    }
}

/// The glass's roles only ever lighten: the floor walks toward the label
/// pole, never across it.
#[test]
fn roles_move_toward_the_label_pole() {
    for id in [ThemeId::GlassClear, ThemeId::GlassNight] {
        let (base, t) = (id.theme(), lift(id.theme()));
        for (was, now) in [(base.dim, t.dim), (base.ansi[2], t.ansi[2])] {
            let (a, b) = (relative_luminance(was), relative_luminance(now));
            assert!(if base.dark { b >= a } else { b <= a }, "{}", id.as_str());
        }
    }
}

/// The tint is the glass's own colour on every desktop: the accent family
/// is not walked (it went navy when it was).
#[test]
fn the_tint_keeps_its_colour() {
    for id in [ThemeId::GlassClear, ThemeId::GlassNight] {
        let (base, t) = (id.theme(), lift(id.theme()));
        assert_eq!(
            (t.accent_default, t.activity, t.broadcast),
            (base.accent_default, base.activity, base.broadcast),
            "{}",
            id.as_str()
        );
    }
}

/// Off glass nothing moves: the served palette's text is the palette's.
#[test]
fn other_palettes_keep_their_roles() {
    for id in ALL_THEMES {
        let base = id.theme();
        if base.liquid.is_some() {
            continue;
        }
        let t = lift(base);
        assert_eq!(
            (t.ink, t.dim, t.ansi),
            (base.ink, base.dim, base.ansi),
            "{}",
            id.as_str()
        );
    }
}

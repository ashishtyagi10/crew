use super::*;
use crate::{ALL_THEMES, CRT_GREEN, PAPER_DARK, PAPER_LIGHT};

#[test]
fn level_round_trips_and_accepts_aliases() {
    for l in [
        GlassLevel::Off,
        GlassLevel::Low,
        GlassLevel::Medium,
        GlassLevel::High,
    ] {
        assert_eq!(GlassLevel::parse(l.as_str()), Some(l));
    }
    assert_eq!(GlassLevel::parse("on"), Some(GlassLevel::Medium));
    assert_eq!(GlassLevel::parse("NONE"), Some(GlassLevel::Off));
    assert_eq!(GlassLevel::parse(" High "), Some(GlassLevel::High));
    assert_eq!(GlassLevel::parse("shiny"), None);
}

/// Liquid glass (2026-09-23): every page that is not a tube wears a sheet,
/// and the sheet casts a shadow — the depth is the point of it.
#[test]
fn every_page_wears_liquid_glass() {
    for id in ALL_THEMES.into_iter().filter(|id| !id.theme().is_tube()) {
        let s = style_for(id.theme()).scaled(GlassLevel::Medium);
        assert!(s.visible(), "{}: no sheet", id.as_str());
        assert!(s.shadow_alpha > 0.0, "{}: no shadow", id.as_str());
        assert!(s.highlight_alpha > 0.0, "{}: no rim", id.as_str());
    }
}

/// A tube is a light construct: no sheet, no shadow, even at High. Its
/// holographic sheet read as a drop shadow that set the panes adrift
/// (2026-08-06), and that verdict stands.
#[test]
fn tubes_stay_flat_even_at_high() {
    let tubes: Vec<_> = ALL_THEMES
        .into_iter()
        .filter(|id| id.theme().is_tube())
        .collect();
    assert!(!tubes.is_empty(), "the filter found no tubes");
    for id in tubes {
        let s = style_for(id.theme()).scaled(GlassLevel::High);
        assert!(!s.visible(), "{}: a tube grew a sheet", id.as_str());
        assert_eq!(
            s.shadow_alpha,
            0.0,
            "{}: a tube casts no shadow",
            id.as_str()
        );
    }
}

/// A bright rim on a dark page reads as a neon outline, and a faint shadow on
/// a near-black page reads as nothing: the dark sheet trades one for the
/// other.
#[test]
fn dark_pages_soften_the_rim_and_deepen_the_shadow() {
    let light = style_for(&PAPER_LIGHT);
    let dark = style_for(&PAPER_DARK);
    assert!(dark.highlight_alpha < light.highlight_alpha * 0.5);
    assert!(dark.shadow_alpha > light.shadow_alpha * 2.0);
    assert!(
        dark.alpha_top < light.alpha_top * 0.5,
        "a dark lift stays faint"
    );
}

#[test]
fn off_draws_nothing() {
    for id in ALL_THEMES {
        assert!(!style_for(id.theme()).scaled(GlassLevel::Off).visible());
    }
}

/// The top edge is always at least as opaque as the bottom — that ramp is
/// what makes the sheet look lit from above.
#[test]
fn fill_is_brightest_at_the_top() {
    for id in ALL_THEMES {
        let s = style_for(id.theme());
        assert!(
            s.alpha_top >= s.alpha_bottom,
            "{} inverts the glass gradient",
            id.as_str()
        );
    }
}

/// Level scaling is monotonic, and no level can resurrect a tube's sheet.
#[test]
fn level_scales_alpha_monotonically() {
    let base = GlassStyle {
        alpha_top: 0.25,
        ..style_for(&CRT_GREEN)
    };
    let low = base.scaled(GlassLevel::Low).alpha_top;
    let med = base.scaled(GlassLevel::Medium).alpha_top;
    let high = base.scaled(GlassLevel::High).alpha_top;
    assert!(low < med && med < high, "{low} {med} {high}");
    for level in [GlassLevel::Low, GlassLevel::Medium, GlassLevel::High] {
        for id in ALL_THEMES.into_iter().filter(|id| id.theme().is_tube()) {
            assert!(
                !style_for(id.theme()).scaled(level).visible(),
                "{}: {} resurrected the sheet",
                id.as_str(),
                level.as_str()
            );
        }
    }
}

/// High strength must not push any alpha past opaque.
#[test]
fn high_never_exceeds_opaque() {
    for id in ALL_THEMES {
        let s = style_for(id.theme()).scaled(GlassLevel::High);
        for a in [
            s.alpha_top,
            s.alpha_bottom,
            s.highlight_alpha,
            s.shadow_alpha,
            s.edge_glow,
        ] {
            assert!((0.0..=1.0).contains(&a), "{} alpha {a}", id.as_str());
        }
    }
}

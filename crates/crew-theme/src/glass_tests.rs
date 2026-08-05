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

/// The whole point of deriving rather than declaring: no theme, present or
/// future, can ship without glass.
#[test]
fn every_theme_gets_visible_glass() {
    for id in ALL_THEMES {
        let s = style_for(id.theme());
        assert!(
            s.scaled(GlassLevel::Medium).visible(),
            "{} has no visible glass",
            id.as_str()
        );
    }
}

#[test]
fn off_draws_nothing() {
    for id in ALL_THEMES {
        assert!(!style_for(id.theme()).scaled(GlassLevel::Off).visible());
    }
}

/// Dark glass is a *lift*: the sheet must be lighter than the page it sits
/// on, or the card reads as a hole rather than a pane of glass.
#[test]
fn dark_glass_lifts_off_the_page() {
    let s = style_for(&PAPER_DARK);
    let lum = |c: (u8, u8, u8)| c.0 as u32 + c.1 as u32 + c.2 as u32;
    assert!(
        lum(s.tint) > lum(PAPER_DARK.page_bg),
        "dark tint {:?} is not lighter than page {:?}",
        s.tint,
        PAPER_DARK.page_bg
    );
}

/// A light page can't get lighter, so light themes must earn their depth
/// with a shadow. Without this the light glass is invisible.
#[test]
fn light_glass_has_a_shadow() {
    assert!(style_for(&PAPER_LIGHT).shadow_alpha > 0.0);
}

/// The holographic contract (goal 2026-08-04), inverted from the old
/// `crt_glass_is_restrained`: CRT glass is a luminous phosphor sheet, MORE
/// opaque than paper-dark's and tinted in the tube's own hue. The two clauses
/// of the old doctrine that were right survive: no grain (the tube grains the
/// frame) and no drop shadow (a light construct casts none).
#[test]
fn crt_glass_is_luminous() {
    let crt = style_for(&CRT_GREEN);
    let dark = style_for(&PAPER_DARK);
    assert!(
        crt.alpha_top > dark.alpha_top,
        "CRT glass ({}) must exceed paper-dark's ({})",
        crt.alpha_top,
        dark.alpha_top
    );
    assert_eq!(crt.noise, 0.0, "the tube already grains the frame");
    assert_eq!(crt.shadow_alpha, 0.0, "a tube has no drop shadows");
    // The tint is measurably phosphor-hued: on the green tube its green
    // channel dominates by a real margin (a grey or white tint fails here)...
    let (r, g, b) = crt.tint;
    assert!(
        g > r + 40 && g > b + 40,
        "tint {:?} is not phosphor-green",
        crt.tint
    );
    // ...and the sheet is a lift of the page, not a darkening.
    let lum = |c: (u8, u8, u8)| c.0 as u32 + c.1 as u32 + c.2 as u32;
    assert!(
        lum(crt.tint) > lum(CRT_GREEN.page_bg),
        "tint {:?} does not lift off page {:?}",
        crt.tint,
        CRT_GREEN.page_bg
    );
    // Only CRT panes are lit by their own frame; paper sheets get no
    // edge-glow, so zero must reach the shader for both paper families.
    assert!(crt.edge_glow > 0.0, "CRT sheet lost its inner edge-glow");
    assert_eq!(dark.edge_glow, 0.0, "paper-dark must not edge-glow");
    assert_eq!(style_for(&PAPER_LIGHT).edge_glow, 0.0);
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

#[test]
fn level_scales_alpha_monotonically() {
    let base = style_for(&PAPER_DARK);
    let low = base.scaled(GlassLevel::Low).alpha_top;
    let med = base.scaled(GlassLevel::Medium).alpha_top;
    let high = base.scaled(GlassLevel::High).alpha_top;
    assert!(low < med && med < high, "{low} {med} {high}");
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

#[test]
fn mix_interpolates_between_endpoints() {
    assert_eq!(mix((0, 0, 0), (255, 255, 255), 0.0), (0, 0, 0));
    assert_eq!(mix((0, 0, 0), (255, 255, 255), 1.0), (255, 255, 255));
    assert_eq!(mix((0, 0, 0), (200, 100, 50), 0.5), (100, 50, 25));
    // Out-of-range factors clamp rather than wrapping around.
    assert_eq!(mix((10, 10, 10), (200, 200, 200), -1.0), (10, 10, 10));
    assert_eq!(mix((10, 10, 10), (200, 200, 200), 2.0), (200, 200, 200));
}

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

/// The 2026-08-06 flat decree, completed: NO family draws a sheet. Paper went
/// first (the derived shadow read as a misaligned duplicate border); CRT
/// followed the same day — its holographic sheet read as a drop shadow that
/// set the panes adrift. Derivation now guarantees every theme, present or
/// future, ships flat.
#[test]
fn no_theme_gets_visible_glass() {
    for id in ALL_THEMES {
        let s = style_for(id.theme()).scaled(GlassLevel::Medium);
        assert!(
            !s.visible(),
            "{}: every theme renders flat — no glass sheet",
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

/// Flat means FLAT: not a faint sheet, but zero everything — even at High.
/// Any non-zero alpha here resurrects the phantom box around the cards.
#[test]
fn every_theme_is_completely_flat_even_at_high() {
    for id in ALL_THEMES {
        let s = style_for(id.theme()).scaled(GlassLevel::High);
        assert!(!s.visible(), "{}: glass must be invisible", id.as_str());
        assert_eq!(s.shadow_alpha, 0.0, "{}: no shadow", id.as_str());
        assert_eq!(s.edge_glow, 0.0, "{}: no edge-glow", id.as_str());
        assert_eq!(s.noise, 0.0, "{}: no frost grain", id.as_str());
    }
}

/// Nothing casts a drop shadow anymore — paper is flat, a CRT light construct
/// casts none. The shadow drawn on the raw pane rect while the frame is
/// cell-quantized is exactly the "weird shadow / misaligned input bar" bug.
#[test]
fn no_theme_casts_a_drop_shadow() {
    for id in ALL_THEMES {
        assert_eq!(
            style_for(id.theme()).shadow_alpha,
            0.0,
            "{} grew a drop shadow back",
            id.as_str()
        );
    }
}

/// The flat-tube decree (2026-08-06, superseding the 2026-08-04 holographic
/// contract): CRT glass is exactly as flat as paper's. The luminous sheet's
/// ramp + specular hairline read as a drop shadow around every pane; what
/// distinguishes a tube from paper-dark now is bloom, border weight and
/// typeface — all carried elsewhere (`CrtStyle`, `border_thickness`,
/// `font_prefs`), none of it here.
#[test]
fn crt_glass_is_flat_like_paper() {
    let alphas = |s: GlassStyle| {
        [
            s.alpha_top,
            s.alpha_bottom,
            s.highlight_alpha,
            s.shadow_alpha,
            s.noise,
            s.edge_glow,
        ]
    };
    let crt = style_for(&CRT_GREEN);
    assert!(!crt.visible());
    assert_eq!(
        alphas(crt),
        alphas(style_for(&PAPER_DARK)),
        "CRT and paper share one flat (all-zero) look"
    );
    assert_eq!(alphas(crt), alphas(style_for(&PAPER_LIGHT)));
    assert_eq!(
        crt.edge_glow, 0.0,
        "the frame lights the border, not a sheet"
    );
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

/// Level scaling still works on a synthetic style (the shader contract), and
/// no level can resurrect a sheet out of the flat derived styles.
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
        for id in ALL_THEMES {
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

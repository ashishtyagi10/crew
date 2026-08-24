use std::hash::{DefaultHasher, Hash, Hasher};

use super::*;

use crate::panecard::{pane_card, Bar};

fn hash_cells(v: &[CellView]) -> u64 {
    let mut h = DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

fn bar(focused: bool) -> Bar<'static> {
    Bar {
        index: Some(2),
        title: "shell",
        focused,
        scroll: 0,
        activity: false,
        bell: false,
        broadcast: false,
        min_btn: false,
        focus_t: 1.0,
        assemble_t: 1.0,
    }
}

fn fg_at(v: &[CellView], col: u16, row: u16) -> (u8, u8, u8) {
    v.iter()
        .find(|c| c.col == col && c.row == row)
        .expect("frame cell exists")
        .fg
}

/// At rest (idle, ignition settled) the ring is an exact corner-to-corner
/// gradient — `pole_a` at the top-left, `pole_b` at the bottom-right, both
/// bit-for-bit — and is independent of the clock, so idle frames stay
/// byte-identical no matter when they are drawn.
#[test]
fn resting_ring_is_the_exact_gradient_and_clock_free() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::Nebula);
    let style = crew_theme::theme().modern.expect("aurora is modern");
    let mut a = pane_card(38, 10, &bar(true));
    ring(&mut a, 40, 12, false, 1.0, 0);
    let mut b = pane_card(38, 10, &bar(true));
    ring(&mut b, 40, 12, false, 1.0, 987_654_321);
    assert_eq!(
        hash_cells(&a),
        hash_cells(&b),
        "an idle ring must not read the clock"
    );
    assert_eq!(fg_at(&a, 0, 0), style.pole_a, "top-left corner is pole_a");
    assert_eq!(
        fg_at(&a, 39, 11),
        style.pole_b,
        "bottom-right corner is pole_b"
    );
    // And it really is a gradient: an edge midpoint sits between the poles,
    // equal to neither.
    let mid = fg_at(&a, 0, 5);
    assert_ne!(mid, style.pole_a);
    assert_ne!(mid, style.pole_b);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

/// A streaming pane's ring drifts: the same cell wears a different colour
/// half a period later (Motion=full), and Motion=off pins it static.
#[test]
fn busy_ring_drifts_and_motion_off_freezes_it() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::Nebula);
    let drift = crew_theme::theme().modern.unwrap().drift_ms;
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let mut a = pane_card(38, 10, &bar(true));
    ring(&mut a, 40, 12, true, 1.0, 0);
    let mut b = pane_card(38, 10, &bar(true));
    ring(&mut b, 40, 12, true, 1.0, drift / 2);
    assert_ne!(
        fg_at(&a, 0, 0),
        fg_at(&b, 0, 0),
        "a busy ring must drift with the clock"
    );
    crate::motion::set_level(crate::motion::MotionLevel::Off);
    let mut c = pane_card(38, 10, &bar(true));
    ring(&mut c, 40, 12, true, 1.0, drift / 2);
    assert_eq!(
        hash_cells(&a),
        hash_cells(&c),
        "Motion=off must pin the ring to its resting gradient"
    );
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

/// Ignition lifts the whole stroke toward white and decays to the exact
/// resting gradient; the legend riding the same border keeps its own colour
/// throughout.
#[test]
fn ignition_lifts_then_settles_and_spares_the_legend() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::Nebula);
    let style = crew_theme::theme().modern.unwrap();
    let mut lit = pane_card(38, 10, &bar(true));
    ring(&mut lit, 40, 12, false, 0.0, 0);
    let corner = fg_at(&lit, 0, 0);
    assert_ne!(corner, style.pole_a, "ignition must lift the stroke");
    let lifted = crate::anim::lerp_rgb(style.pole_a, (255, 255, 255), IGNITE_LIFT);
    assert_eq!(corner, lifted, "the lift is IGNITE_LIFT toward white");
    // Settled: exact rest.
    let mut rest = pane_card(38, 10, &bar(true));
    ring(&mut rest, 40, 12, false, 1.0, 0);
    assert_eq!(fg_at(&rest, 0, 0), style.pole_a);
    // The legend keeps its colour: compare non-stroke cells against an
    // untraced card.
    let plain = pane_card(38, 10, &bar(true));
    for (p, l) in plain.iter().zip(lit.iter()) {
        if !is_frame_glyph(p.c) || p.fg != crew_theme::theme().border_focused {
            assert_eq!(p.fg, l.fg, "non-stroke cell {:?} was recoloured", p.c);
        }
    }
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

/// On a theme without a `ModernStyle` the ring is a strict no-op.
#[test]
fn every_theme_draws_the_ring_now() {
    let _g = crate::app::theme_test_guard();
    // This test used to assert the opposite — that the ring was a no-op anywhere outside the
    // two-theme modern family. Every palette carries a gradient now, so the guard worth having
    // is that none of them LOST one: a preset whose `modern` went back to `None` would silently
    // stop glowing, and nothing else would notice.
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let plain = pane_card(38, 10, &bar(true));
        let mut v = pane_card(38, 10, &bar(true));
        ring(&mut v, 40, 12, true, 0.0, 12_345);
        assert_ne!(
            hash_cells(&plain),
            hash_cells(&v),
            "{} has no gradient ring",
            id.as_str()
        );
    }
}

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
        total: 0,
        activity: false,
        bell: false,
        broadcast: false,
        min_btn: false,
        focus_t: 1.0,
        assemble_t: 1.0,
        git: None,
        ticks: &[],
        hits: &[],
        progress: None,
        elapsed: None,
        unread: 0,
        doc: false,
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

/// Luma in the same gamma space `at_luma_of` measures.
fn luma(c: (u8, u8, u8)) -> f32 {
    0.2126 * f32::from(c.0) + 0.7152 * f32::from(c.1) + 0.0722 * f32::from(c.2)
}

/// The stroke cells of a card, in the order they were drawn.
fn stroke(v: &[CellView]) -> Vec<(u8, u8, u8)> {
    v.iter()
        .filter(|c| is_frame_glyph(c.c))
        .map(|c| c.fg)
        .collect()
}

/// An UNFOCUSED card is no longer a flat stroke on any theme: its border runs
/// a real gradient — many distinct colours, different at the two corners —
/// which is the whole point of pushing the gradient past the focused pane.
#[test]
fn every_theme_tints_an_unfocused_card() {
    let _g = crate::app::theme_test_guard();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let flat = crew_theme::theme().border_normal;
        let v = pane_card(38, 10, &bar(false));
        let cols = stroke(&v);
        assert!(!cols.is_empty(), "{} drew no stroke", id.as_str());
        let flats = cols.iter().filter(|&&c| c == flat).count();
        assert!(
            flats * 4 < cols.len(),
            "{}: {flats}/{} stroke cells are still the flat border",
            id.as_str(),
            cols.len()
        );
        let distinct: std::collections::HashSet<_> = cols.iter().collect();
        assert!(
            distinct.len() >= 6,
            "{}: only {} distinct stroke colours — that is a tint, not a gradient",
            id.as_str(),
            distinct.len()
        );
        assert_ne!(
            fg_at(&v, 0, 0),
            fg_at(&v, 39, 11),
            "{}: the two corners must not match",
            id.as_str()
        );
    }
}

/// …and it pays for that colour with no brightness. Every quiet cell sits
/// within a hair of `border_normal`'s luminance on every theme, which is what
/// stops nine coloured frames from flattening the canvas: hue travels, level
/// does not.
#[test]
fn a_quiet_stroke_holds_the_flat_luminance() {
    let _g = crate::app::theme_test_guard();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let want = luma(crew_theme::theme().border_normal);
        for c in stroke(&pane_card(38, 10, &bar(false))) {
            let got = luma(c);
            assert!(
                (got - want).abs() <= 2.0,
                "{}: stroke {c:?} at luma {got:.1}, flat border is {want:.1}",
                id.as_str()
            );
        }
    }
}

/// Hierarchy survives. The focused pane's ring stands further off the page
/// than any quiet card's stroke on every theme — measured as CONTRAST against
/// `page_bg`, not brightness: on a light theme prominence is darker ink, not
/// more light, and a "the focused frame is the brightest" rule would be a
/// dark-theme assumption wearing a general name.
///
/// Focus brackets are switched off (`focus_t = 0`) because they are painted
/// in the palette accent over the same frame glyphs — measuring them would
/// tell us about `palette::accent`, not about the gradient.
#[test]
fn the_focused_ring_stands_further_off_the_page_than_any_quiet_stroke() {
    let _g = crate::app::theme_test_guard();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let page = crew_theme::theme().page_bg;
        let peak = |v: &[CellView]| {
            stroke(v)
                .into_iter()
                .map(|c| crew_theme::contrast_ratio(c, page))
                .fold(0.0, f32::max)
        };
        let mut b = bar(true);
        b.focus_t = 0.0;
        let mut lit = pane_card(38, 10, &b);
        ring(&mut lit, 40, 12, false, 1.0, 0);
        let hot = peak(&lit);
        let mut q = bar(false);
        q.focus_t = 0.0;
        let cool = peak(&pane_card(38, 10, &q));
        assert!(
            hot > cool * 1.5,
            "{}: focused ring reaches {hot:.2}:1 against the page, a quiet card {cool:.2}:1",
            id.as_str()
        );
    }
}

/// The quiet pass keeps the ring's contract: only frame glyphs are touched.
/// The legend and the status glyphs riding the same border rows are left
/// exactly as drawn — the gradient is chrome, and it does not get to repaint
/// a signal.
#[test]
fn a_quiet_stroke_spares_the_legend_and_the_status_glyphs() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::Nebula);
    let mut b = bar(false);
    b.activity = true;
    b.bell = true;
    let t = crew_theme::theme();
    let v = pane_card(38, 10, &b);
    // The legend keeps the pane's signature hue, receded toward `legend_off`
    // exactly as `pane_card` derived it.
    let want_legend =
        crate::anim::lerp_rgb(crate::chatroster::agent_color("shell"), t.legend_off, 0.55);
    let legend: Vec<_> = v
        .iter()
        .filter(|c| c.row == 0 && c.c == 'h')
        .map(|c| c.fg)
        .collect();
    assert_eq!(legend, vec![want_legend], "the legend lost its hue");
    // The status glyphs are still their own signal colours.
    for (glyph, want) in [('\u{25cf}', t.activity), ('!', t.bell)] {
        let got = v
            .iter()
            .find(|c| c.c == glyph)
            .unwrap_or_else(|| panic!("{glyph} drawn"))
            .fg;
        assert_eq!(got, want, "{glyph} lost its colour to the gradient");
    }
    // And the frame really did move off the flat colour.
    assert_ne!(fg_at(&v, 0, 1), t.border_normal);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

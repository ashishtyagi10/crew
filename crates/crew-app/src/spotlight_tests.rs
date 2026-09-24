use super::*;

#[test]
fn at_rest_only_the_spotlit_pane_holds_full_ink() {
    assert_eq!(dim_for(2, 2, 0, 1.0), 0.0);
    assert_eq!(dim_for(0, 2, 0, 1.0), DIM, "the pane focus left is dim");
    assert_eq!(dim_for(1, 2, 0, 1.0), DIM, "bystanders are dim");
}

#[test]
fn focus_travel_crossfades_old_and_new() {
    // Mid-travel: the new pane is half-lit, the old half-dimmed, and the
    // two strengths mirror each other exactly.
    let up = dim_for(2, 2, 0, 0.5);
    let down = dim_for(0, 2, 0, 0.5);
    assert!((up - DIM * 0.5).abs() < 1e-6);
    assert!((up - down).abs() < 1e-6);
    // At the start of travel the roles are fully swapped.
    assert_eq!(dim_for(2, 2, 0, 0.0), DIM);
    assert_eq!(dim_for(0, 2, 0, 0.0), 0.0);
}

#[test]
fn wash_moves_ink_toward_the_page_but_leaves_backgrounds() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    let mut cells = vec![CellView {
        col: 0,
        row: 0,
        c: 'x',
        fg: t.ink,
        bg: (10, 20, 30),
        bold: false,
        italic: false,
        ..Default::default()
    }];
    let before = cells[0].fg;
    wash(&mut cells, DIM);
    assert_ne!(cells[0].fg, before, "ink must move");
    assert_eq!(cells[0].bg, (10, 20, 30), "backgrounds must not");
    assert_eq!(
        cells[0].fg,
        crate::anim::lerp_rgb(before, t.page_bg, DIM),
        "wash is exactly the documented lean"
    );
    // Zero dim is a strict no-op.
    let unwashed = cells[0].fg;
    wash(&mut cells, 0.0);
    assert_eq!(cells[0].fg, unwashed);
}

/// The spotlit card rises on the focus clock, the one it left settles on the
/// same clock, and the rest never leave the page — so at every instant of a
/// focus move exactly one full lift is shared between two cards.
#[test]
fn lift_hands_over_between_the_two_cards_on_one_clock() {
    for t in [0.0, 0.3, 0.7, 1.0] {
        let (spot, prev, other) = (
            lift_for(2, 2, 0, t),
            lift_for(0, 2, 0, t),
            lift_for(1, 2, 0, t),
        );
        assert_eq!(spot, t);
        assert_eq!(spot + prev, 1.0, "one lift shared at t={t}");
        assert_eq!(other, 0.0, "a bystander rests at t={t}");
    }
    // Focus never moved (spot == prev): the card is simply up.
    assert_eq!(lift_for(1, 1, 1, 1.0), 1.0);
}

/// The input bar's scene in a real frame: raised, never sunk, and higher
/// while you type than any pane — the focused one included.
#[test]
fn the_input_bar_floats_above_the_panes() {
    let lift_of = |typing: bool| {
        let mut app = crate::app::CrewApp {
            geo_override: Some((8.0, 17.0, 1280.0, 720.0, 1.0)),
            ..Default::default()
        };
        app.submit_input("/far".to_string());
        app.input.focused = typing;
        let scenes = app.build_frame();
        let bar = scenes
            .iter()
            .filter(|s| s.glass && s.stretch && !s.bordered && !s.overlay)
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .expect("an input bar scene");
        let panes = scenes.iter().filter(|s| s.bordered).map(|s| s.lift);
        (bar.lift, panes.fold(0.0_f32, f32::max))
    };
    let (rest, _) = lift_of(false);
    let (typing, pane) = lift_of(true);
    assert!(
        rest >= 1.0,
        "at rest the bar rides like a focused card: {rest}"
    );
    assert!(typing > rest, "typing lifts it further: {typing} vs {rest}");
    assert!(
        typing > pane,
        "above every pane while typing: {typing} vs {pane}"
    );
}

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

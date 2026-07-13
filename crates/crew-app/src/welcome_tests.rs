use super::*;

#[test]
fn welcome_cells_in_bounds() {
    let cells = welcome_cells_animated(80, 24, 7, None);
    assert!(!cells.is_empty());
    assert!(
        cells.iter().all(|c| c.col < 80 && c.row < 24),
        "cell out of 80×24 bounds"
    );
}

#[test]
fn hint_present() {
    let cells = welcome_cells_animated(80, 24, 0, None);
    let hint_fg = crew_theme::theme().hint_fg;
    assert!(
        cells.iter().any(|c| c.fg == hint_fg),
        "no hint_fg cells in welcome output"
    );
}

#[test]
fn version_stamp_present() {
    let cells = welcome_cells_animated(80, 24, 0, None);
    let dim = crew_theme::theme().dim;
    assert!(
        cells
            .iter()
            .any(|c| c.c == 'v' && c.row == 23 && c.fg == dim),
        "no version stamp on bottom row"
    );
}

#[test]
fn tiny_size_no_panic_and_in_bounds() {
    let cells = welcome_cells_animated(2, 1, 0, None);
    assert!(cells.iter().all(|c| c.col < 2 && c.row < 1));
}

#[test]
fn empty_screen_produces_cells() {
    assert!(!welcome_cells_animated(80, 24, 0, None).is_empty());
}

#[test]
fn anim_redraws_one_in_every_anim_div_ticks() {
    let redraws = (0..ANIM_DIV * 4).filter(|&t| anim_should_redraw(t)).count();
    assert_eq!(redraws as u64, 4, "one redraw per ANIM_DIV ticks");
    assert!(anim_should_redraw(0) && anim_should_redraw(ANIM_DIV));
    assert!(!anim_should_redraw(1));
}

#[test]
fn globe_width_picks_the_default_size_when_roomy() {
    assert_eq!(globe_width(90, 30), Some(44));
}

#[test]
fn globe_width_scales_down_to_fit_the_rows() {
    // Default 44x22 needs rows > 25 (22 + 3); at rows=24 it steps down to 40x20.
    assert_eq!(globe_width(90, 24), Some(40));
}

#[test]
fn globe_width_falls_back_when_nothing_fits() {
    assert_eq!(
        globe_width(10, 24),
        None,
        "too narrow for even the min width"
    );
    assert_eq!(
        globe_width(90, 10),
        None,
        "too short for even the min height"
    );
}

#[test]
fn globe_sits_above_tagline_and_hint() {
    let cells = welcome_cells_animated(80, 30, 0, None);
    let t = crew_theme::theme();
    let globe_max_row = cells
        .iter()
        .filter(|c| c.fg == t.ink || c.fg == t.text_muted)
        .map(|c| c.row)
        .max()
        .expect("expected globe cells");
    let hint_min_row = cells
        .iter()
        .filter(|c| c.fg == t.hint_fg)
        .map(|c| c.row)
        .min()
        .expect("expected tagline/hint cells");
    assert!(
        globe_max_row < hint_min_row,
        "globe rows must sit above the tagline/hint"
    );
}

#[test]
fn welcome_animates_over_time() {
    let a = welcome_cells_animated(80, 30, 0, None);
    let b = welcome_cells_animated(80, 30, 20, None);
    let chars = |v: &[CellView]| {
        v.iter()
            .map(|c| (c.col, c.row, c.c, c.fg))
            .collect::<Vec<_>>()
    };
    assert_ne!(
        chars(&a),
        chars(&b),
        "the globe frame must change over time"
    );
}

fn text(cells: &[CellView]) -> String {
    let mut v: Vec<_> = cells.iter().collect();
    v.sort_by_key(|c| (c.row, c.col));
    v.iter().map(|c| c.c).collect()
}

#[test]
fn restore_hint_renders_below_the_keyboard_hint() {
    let with = welcome_cells_animated(80, 30, 0, Some(3));
    assert!(
        text(&with).contains("3 shells from last session"),
        "{}",
        text(&with)
    );
    assert!(text(&with).contains("/restore"));
    let without = welcome_cells_animated(80, 30, 0, None);
    assert!(!text(&without).contains("/restore"));
}

#[test]
fn restore_hint_singular_and_in_bounds_on_tight_rows() {
    let one = welcome_cells_animated(80, 30, 0, Some(1));
    assert!(text(&one).contains("1 shell from last session"));
    // Rows exactly at the globe stack budget: the extra row is clipped, not
    // drawn out of bounds.
    let tight = welcome_cells_animated(80, 24, 0, Some(2));
    assert!(tight.iter().all(|c| c.row < 24 && c.col < 80));
}

#[test]
fn restore_hint_never_shares_the_version_stamp_row() {
    // cols=50 rows=28 puts the naive restore row exactly on rows-1, where
    // the centred line's tail met "v0.x.y" (last-write-wins garbling) —
    // review-found collision band. The row is skipped there instead.
    let cells = welcome_cells_animated(50, 28, 0, Some(3));
    let stamp_row = 27u16;
    let hint_fg = crew_theme::theme().hint_fg;
    assert!(
        cells.iter().all(|c| c.row != stamp_row || c.fg != hint_fg),
        "no hint-coloured cells may share the version stamp row"
    );
}

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
fn rain_width_picks_the_default_size_when_roomy() {
    assert_eq!(rain_width(90, 30), Some(64));
}

#[test]
fn rain_width_scales_down_to_fit_the_rows() {
    // Default 64x16 needs rows > 19 (16 + 3); at rows=18 it steps down to
    // 58x14 (the first even width whose h+3 stack fits).
    assert_eq!(rain_width(90, 18), Some(58));
}

#[test]
fn rain_width_falls_back_when_nothing_fits() {
    assert_eq!(
        rain_width(10, 24),
        None,
        "too narrow for even the min width"
    );
    assert_eq!(rain_width(90, 8), None, "too short for even the min height");
}

#[test]
fn rain_sits_above_tagline_and_hint() {
    let cells = welcome_cells_animated(80, 30, 0, None);
    let t = crew_theme::theme();
    let rain_max_row = cells
        .iter()
        .filter(|c| c.fg == t.ink || c.fg == t.text_muted)
        .map(|c| c.row)
        .max()
        .expect("expected rain cells");
    let hint_min_row = cells
        .iter()
        .filter(|c| c.fg == t.hint_fg)
        .map(|c| c.row)
        .min()
        .expect("expected tagline/hint cells");
    assert!(
        rain_max_row < hint_min_row,
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
    assert_ne!(chars(&a), chars(&b), "the rain frame must change over time");
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
        text(&with).contains("3 panes from last session"),
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
    assert!(text(&one).contains("1 pane from last session"));
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

#[test]
fn rain_box_is_framed_with_an_inner_crew_nameplate() {
    let cells = welcome_cells_animated(80, 30, 0, None);
    let chars: std::collections::HashSet<char> = cells.iter().map(|c| c.c).collect();
    // The rectangular frame's corners…
    for c in ['\u{250c}', '\u{2510}', '\u{2514}', '\u{2518}'] {
        assert!(chars.contains(&c), "frame corner {c} missing");
    }
    // …and the double-line CREW nameplate over the rain, letters in bold.
    for c in ['\u{2554}', '\u{255d}'] {
        assert!(chars.contains(&c), "nameplate corner {c} missing");
    }
    for l in ['C', 'R', 'E', 'W'] {
        assert!(
            cells.iter().any(|c| c.c == l && c.bold),
            "nameplate letter {l} missing"
        );
    }
    // The rain stays inside the frame: no glyph cells on the frame's ring is
    // hard to assert cheaply, but everything must stay in bounds.
    assert!(cells.iter().all(|c| c.row < 30 && c.col < 80));
}

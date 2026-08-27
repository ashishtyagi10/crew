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
    // The rows are told apart BY COLOUR, so the theme must not change under
    // the test: another test switching palettes mid-run makes `t` disagree
    // with the cells that were just built, and the filters come back empty.
    let _g = crate::app::theme_test_guard();
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
        "globe rows must sit above the tagline/hint (rain {rain_max_row}, hint {hint_min_row})"
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

/// The one piece of guidance a first run gets must name the agents. crew is
/// not just a terminal, and a welcome screen that says "shell" and "commands"
/// and nothing else is a welcome screen nobody finds the agents from.
#[test]
fn the_welcome_hint_names_the_agent_pane() {
    for cols in [30u16, 40, 60, 80, 120] {
        let hint = super::hint_for(cols).unwrap_or_else(|| panic!("no hint fits {cols}"));
        assert!(
            (hint.chars().count() as u16) < cols,
            "{cols}: hint is {} wide: {hint}",
            hint.chars().count()
        );
        assert!(hint.contains("Cmd+J"), "{cols}: agents unmentioned: {hint}");
    }
}

/// Below the shortest form there is genuinely nothing to say, and saying it
/// badly is worse than the blank.
#[test]
fn a_pane_too_narrow_for_any_hint_gets_none() {
    assert!(super::hint_for(8).is_none());
}

/// Wider windows get the roomier form — the ladder must be ordered widest
/// first, or a 120-column window would show the cramped one.
#[test]
fn a_wide_window_gets_the_roomiest_form() {
    let wide = super::hint_for(120).unwrap();
    let narrow = super::hint_for(45).unwrap();
    assert!(
        wide.chars().count() > narrow.chars().count(),
        "wide {wide:?} is not roomier than narrow {narrow:?}"
    );
}

/// Crew ships often and every release's headline is compiled in already; a
/// first frame that says what is new is how any of it gets found.
#[test]
fn the_welcome_names_what_this_build_brought() {
    let _g = crate::app::theme_test_guard();
    let line = super::whats_new(120).expect("a headline for this build");
    assert!(line.starts_with("new in "), "{line}");
    let version = env!("CARGO_PKG_VERSION");
    assert!(line.contains(version), "{line} does not name {version}");
    // It is one line of prose, not a paragraph of one.
    assert!(!line.contains('\n'));
    assert!(line.chars().count() <= 120);
    // …and it is on the screen.
    let cells = welcome_cells_animated(120, 30, 0, None);
    let text = text(&cells);
    assert!(
        text.contains("new in "),
        "the headline never reached the frame"
    );
}

/// A window too narrow to say anything useful gets nothing; a headline
/// longer than a usable window is clipped, since its length is the
/// changelog's doing rather than the window's.
#[test]
fn a_narrow_window_drops_the_headline_and_a_long_one_is_clipped() {
    assert_eq!(
        super::whats_new(10),
        None,
        "no room for a version and a word"
    );
    assert_eq!(super::whats_new(24), None);
    let wide = super::whats_new(400).expect("a headline at any sane width");
    let narrow = super::whats_new(60).expect("60 columns is not narrow");
    assert!(narrow.chars().count() <= 60);
    assert!(narrow.starts_with("new in "));
    if wide.chars().count() > 60 {
        assert!(narrow.ends_with('\u{2026}'), "{narrow}");
    }
}

use super::*;

/// The glyphs of `row`, left to right.
fn text(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == row).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect()
}

#[test]
fn a_rate_is_four_columns_at_every_speed_a_link_can_run() {
    // The arrow beside it owns the fifth column; a rate that takes five is a
    // rate with its unit clipped off, which is a different number.
    for b in [
        0,
        1,
        999,
        1_000,
        1_023,
        1_024,
        999_999,
        1_048_576,
        10 * 1_048_576,
        999 * 1_048_576,
    ] {
        let s = short_rate(b);
        assert!(s.chars().count() <= 4, "{b} B/s drew as {s:?}");
        assert!(s.ends_with(['B', 'K', 'M']), "{b} B/s lost its unit: {s:?}");
    }
}

#[test]
fn a_rate_rounds_up_into_the_next_unit_rather_than_growing_a_column() {
    // 1023 B is "1023B" (5), so the boundary has to move below it.
    assert_eq!(short_rate(1_020), "1K");
    assert_eq!(short_rate(999), "999B");
    assert_eq!(short_rate(1_048_576), "1.0M");
    assert_eq!(short_rate(11 * 1_048_576), "11M");
}

#[test]
fn both_directions_get_a_row_and_the_arrows_are_told_apart_by_colour() {
    let mut cells = Vec::new();
    net(&mut cells, 4096, 0, 7, 5);
    assert_eq!(text(&cells, 7), "\u{2193}4K");
    assert_eq!(text(&cells, 8), "\u{2191}0B");
    let arrows: Vec<(u8, u8, u8)> = cells
        .iter()
        .filter(|c| c.c == '\u{2193}' || c.c == '\u{2191}')
        .map(|c| c.fg)
        .collect();
    assert_eq!(arrows.len(), 2);
    assert_ne!(arrows[0], arrows[1], "down and up are not the same colour");
}

#[test]
fn a_dirty_tree_says_how_dirty_and_a_clean_one_says_so_in_one_glyph() {
    let mut cells = Vec::new();
    git(
        &mut cells,
        Some(&GitInfo {
            branch: "main".into(),
            changed: 9,
            ahead: 1,
            behind: 2,
        }),
        3,
        5,
    );
    assert_eq!(text(&cells, 3), "\u{25cf} 9");
    assert_eq!(text(&cells, 4), "\u{2191}1 \u{2193}2");

    let mut clean = Vec::new();
    git(
        &mut clean,
        Some(&GitInfo {
            branch: "main".into(),
            changed: 0,
            ahead: 0,
            behind: 0,
        }),
        3,
        5,
    );
    assert_eq!(text(&clean, 3), "\u{2713}");
    assert!(text(&clean, 4).is_empty(), "no ↑↓ row with nothing to say");
}

#[test]
fn a_tree_that_is_not_a_repo_draws_nothing() {
    let mut cells = Vec::new();
    git(&mut cells, None, 3, 5);
    assert!(cells.is_empty());
}

#[test]
fn each_system_reading_gets_a_letter_and_a_drawn_meter() {
    let _g = crate::app::theme_test_guard();
    let mut cells = Vec::new();
    let mut paint = Vec::new();
    let s = Stats {
        cpu: 0.2,
        mem: 0.6,
        disk: 0.9,
        ..Default::default()
    };
    sys(&mut cells, &mut paint, s, 4, 5, 2.0);
    assert_eq!(
        (text(&cells, 4), text(&cells, 5), text(&cells, 6)),
        ("C".into(), "M".into(), "D".into())
    );
    assert!(!paint.is_empty(), "the meters are drawn, not spelled");
    // Every quad lands inside the three rows the block was given, and clear of
    // the letter column.
    for p in &paint {
        assert!(p.x >= 1.0, "a meter reached back into the letter: {}", p.x);
        assert!(p.x + p.w <= 5.0, "a meter ran past the rail's last column");
        assert!(p.y >= 4.0 && p.y + p.h <= 7.0, "paint outside the block");
    }
    // A full disk and an idle CPU are not drawn in one colour: the tier is
    // said by hue here exactly as it is on the open nav's gauges.
    assert_ne!(
        crate::gauges::fill_color(0.2),
        crate::gauges::fill_color(0.9)
    );
}

#[test]
fn a_count_too_wide_for_the_rail_is_never_drawn_as_its_leading_digits() {
    for (changed, ahead) in [(1_234usize, 0usize), (12usize, 3456usize)] {
        let mut cells = Vec::new();
        git(
            &mut cells,
            Some(&GitInfo {
                branch: "main".into(),
                changed,
                ahead,
                behind: 0,
            }),
            0,
            5,
        );
        for row in [0, 1] {
            let drawn = text(&cells, row);
            let digits: String = drawn.chars().filter(char::is_ascii_digit).collect();
            assert!(
                digits.is_empty() || digits == changed.to_string() || digits == ahead.to_string(),
                "row {row} drew {drawn:?} — a piece of a number, which is a different number"
            );
        }
    }
}

#[test]
fn a_count_that_fits_is_drawn_whole_and_tightens_before_it_gives_up() {
    let mut cells = Vec::new();
    git(
        &mut cells,
        Some(&GitInfo {
            branch: "main".into(),
            changed: 1234,
            ahead: 12,
            behind: 34,
        }),
        0,
        6,
    );
    // Six columns: `● 1234` fits, and the pair tightens to `↑12↓34`.
    assert_eq!(text(&cells, 0), "\u{25cf} 1234");
    assert_eq!(text(&cells, 1), "\u{2191}12\u{2193}34");
}

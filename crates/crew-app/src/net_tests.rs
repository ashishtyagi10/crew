use super::*;

#[test]
fn rate_units() {
    assert_eq!(rate(0), "0 B/s");
    assert_eq!(rate(500), "500 B/s");
    assert_eq!(rate(2048), "2 KB/s");
    assert_eq!(rate(3_500_000), "3.3 MB/s");
}

#[test]
fn net_section_has_rule_and_both_rates() {
    // The colours are derived from the live theme now, so two reads of
    // the process global must not straddle another test switching it.
    let _g = crate::app::theme_test_guard();
    let cells = net_cells(2048, 1024, 64 * 1024, 24);
    assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
    assert!(!cells.iter().any(|c| c.c == '╭'));
    // both rates share row 1
    assert!(cells.iter().any(|c| c.c == '↓' && c.row == 1));
    assert!(cells.iter().any(|c| c.c == '↑' && c.row == 1));
    // and the chart rows below carry no glyphs at all: the twin chart is
    // drawn on the paint layer, and a leftover block ramp here would show
    // through it.
    assert!(!cells.iter().any(|c| c.row >= 2));
}

/// A nav too narrow for both rates shows the busier direction whole, not
/// both of them cut. `↑ 0 B` is a different unit, not a smaller reading.
#[test]
fn a_narrow_nav_drops_a_direction_rather_than_its_unit() {
    let _g = crate::app::theme_test_guard();
    let row = |cols| -> String {
        let mut v: Vec<_> = net_cells(4_000_000, 900, 64 * 1024, cols)
            .into_iter()
            .filter(|c| c.row == 1)
            .collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert_eq!(row(28), "↓ 3.8 MB/s  ↑ 900 B/s");
    assert_eq!(row(24), "↓ 3.8 MB/s ↑ 900 B/s");
    // Down is carrying more, so down is what survives — whole.
    assert_eq!(row(18), "↓ 3.8 MB/s");
    assert!(!row(18).contains('…'), "a value is dropped, never clipped");
}

/// The twin chart's ceiling moves with the traffic, so the rule writes it
/// down. A shape with a moving ceiling and no ceiling written down is a
/// shape you cannot read a value off.
#[test]
fn the_rule_names_the_scale_the_chart_is_drawn_against() {
    let _g = crate::app::theme_test_guard();
    let rule = |ceiling| -> String {
        let mut v: Vec<_> = net_cells(0, 0, ceiling, 28)
            .into_iter()
            .filter(|c| c.row == 0 && c.c != '─')
            .collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect::<String>().trim().to_string()
    };
    assert_eq!(rule(64 * 1024), "NET peak 64 KB/s");
    assert_eq!(rule(9_000_000), "NET peak 8.6 MB/s");
}

/// Dragged wide, the two rates go to opposite ends of the row — they are
/// opposite directions, and a wide nav used to read as one short run at
/// the left with twenty columns of nothing after it.
#[test]
fn a_wide_nav_puts_the_two_directions_at_opposite_ends() {
    let _g = crate::app::theme_test_guard();
    let cells = net_cells(4_000_000, 900, 64 * 1024, 38);
    let row: Vec<_> = {
        let mut v: Vec<_> = cells.iter().filter(|c| c.row == 1).collect();
        v.sort_by_key(|c| c.col);
        v
    };
    assert_eq!(row.first().map(|c| (c.col, c.c)), Some((3, '↓')));
    assert_eq!(row.last().map(|c| c.col), Some(36), "up ends at the edge");
    assert!(row.iter().any(|c| c.c == '↑'));
    // …but at the default width they stay together: two readings six
    // columns apart look spread by accident, not on purpose.
    let mut tight: Vec<_> = net_cells(4_000_000, 900, 64 * 1024, 26)
        .into_iter()
        .filter(|c| c.row == 1)
        .collect();
    tight.sort_by_key(|c| c.col);
    let gap = tight
        .windows(2)
        .map(|w| w[1].col - w[0].col)
        .max()
        .unwrap_or(0);
    assert!(gap <= 2, "still one run: biggest gap {gap} columns");
}

/// …and when it is the upload that is busy, the upload is what survives.
#[test]
fn the_direction_that_survives_is_the_one_carrying_more() {
    let _g = crate::app::theme_test_guard();
    let mut v: Vec<_> = net_cells(900, 4_000_000, 64 * 1024, 18)
        .into_iter()
        .filter(|c| c.row == 1)
        .collect();
    v.sort_by_key(|c| c.col);
    let row: String = v.iter().map(|c| c.c).collect();
    assert_eq!(row, "↑ 3.8 MB/s");
}

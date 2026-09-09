use super::*;

#[test]
fn clock_section_has_rule_and_centered_time() {
    let cells = clock_cells("14:03:09", "Sat 21 Jun", None, 24);
    // horizontal rule, not a box
    assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
    assert!(!cells.iter().any(|c| c.c == '╭'));
    // TIME legend on the divider row
    assert!(cells.iter().any(|c| c.c == 'T' && c.row == 0));
    // time digits on row 1
    assert!(cells.iter().any(|c| c.c == '1' && c.row == 1));
}

#[test]
fn narrow_card_renders_nothing() {
    assert!(clock_cells("12:00:00", "Mon", None, 6).is_empty());
}

/// The weather strip stands in the gap row, muted and centered; without a
/// place set the row stays empty.
#[test]
fn the_weather_strip_takes_the_gap_row() {
    let _g = crate::app::theme_test_guard();
    let cells = clock_cells("14:03:09", "Sat 21 Jun", Some("\u{2600} 24\u{00b0}"), 24);
    let row3: String = {
        let mut v: Vec<&crew_render::CellView> = cells.iter().filter(|c| c.row == 3).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert_eq!(row3, "\u{2600} 24\u{00b0}");
    assert!(cells
        .iter()
        .filter(|c| c.row == 3)
        .all(|c| c.fg == crew_theme::theme().text_muted));
    let none = clock_cells("14:03:09", "Sat 21 Jun", None, 24);
    assert!(none.iter().all(|c| c.row < 3));
}

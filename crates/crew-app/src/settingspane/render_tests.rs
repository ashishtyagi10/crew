use super::*;
use crate::config::CrewConfig;
use crate::settingspane::FIELDS;

fn pane() -> SettingsPane {
    SettingsPane::new(CrewConfig::default(), Vec::new())
}

fn row_text(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == row)
        .map(|c| (c.col, c.c))
        .collect();
    v.sort_unstable();
    // Gap-preserving: pad to each cell's column (blank cells are not emitted).
    let mut s = String::new();
    for (col, c) in v {
        while s.chars().count() < col as usize {
            s.push(' ');
        }
        s.push(c);
    }
    s
}

fn dump(cells: &[CellView], rows: u16) -> String {
    (0..rows).map(|r| row_text(cells, r) + "\n").collect()
}

#[test]
fn every_field_renders_on_a_tall_pane() {
    let cells = pane().cells(80, 30);
    let all = dump(&cells, 30);
    for f in FIELDS.iter().take(FIELDS.len() - 2) {
        assert!(
            all.contains(label_of(*f)),
            "missing field '{}' in:\n{all}",
            label_of(*f)
        );
    }
    assert!(all.contains("[ Save \u{2318}S ]"), "save button: {all}");
    assert!(all.contains("[ Cancel esc ]"), "cancel button: {all}");
}

#[test]
fn cards_have_legends() {
    let all = dump(&pane().cells(80, 30), 30);
    for t in ["APPEARANCE", "WINDOW", "NOTIFICATIONS"] {
        assert!(all.contains(t), "missing card '{t}' in:\n{all}");
    }
}

#[test]
fn focused_input_carries_cursor() {
    // Focus starts on FontFamily; its box content row carries the cursor.
    let all = dump(&pane().cells(80, 30), 30);
    assert!(all.contains('\u{2588}'), "cursor missing:\n{all}");
}

#[test]
fn short_pane_scrolls_to_keep_focus_visible() {
    let mut p = pane();
    p.focus = FIELDS
        .iter()
        .position(|&f| f == Field::NotifyPatterns)
        .unwrap();
    let cells = p.cells(80, 12);
    let all = dump(&cells, 12);
    assert!(all.contains("Patterns"), "focused field visible:\n{all}");
    assert!(all.contains('\u{2191}'), "up hint expected:\n{all}");
}

#[test]
fn narrow_pane_still_renders_all_cards() {
    let all = dump(&pane().cells(48, 60), 60);
    for t in ["APPEARANCE", "WINDOW", "NOTIFICATIONS"] {
        assert!(all.contains(t), "missing card '{t}' in:\n{all}");
    }
}

#[test]
fn tiny_pane_renders_nothing() {
    assert!(pane().cells(10, 4).is_empty());
}

#[test]
fn theme_value_names_the_current_theme() {
    // An unset config now labels as the default consolidated mode, `dark`.
    let (v, cursor) = value_of(&pane(), Field::Theme);
    assert!(v.contains("dark"), "got: {v}");
    assert!(!cursor, "theme is a picker, not a text field");
}

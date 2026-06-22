use super::*;
use crate::config::CrewConfig;

#[test]
fn renders_bordered_input_boxes() {
    let p = SettingsPane::new(CrewConfig::default(), Vec::new());
    let cells = p.cells(60, 20);
    // ratatui rounded blocks → real corner glyphs, not [ ] brackets
    assert!(cells.iter().any(|c| c.c == '╭'));
    assert!(cells.iter().any(|c| c.c == '╰'));
    // field legend renders in the top border
    assert!(cells.iter().any(|c| c.c == 'F'));
}

#[test]
fn tiny_pane_renders_nothing() {
    let p = SettingsPane::new(CrewConfig::default(), Vec::new());
    assert!(p.cells(10, 4).is_empty());
}

#[test]
fn wide_pane_uses_two_columns() {
    let (fr, _) = field_layout(Rect::new(0, 0, 100, 20));
    // family (fr[0]) and nav (fr[2]) sit in different columns…
    assert!(fr[2].x > fr[0].x);
    // …while family + size share the left column.
    assert_eq!(fr[0].x, fr[1].x);
}

#[test]
fn narrow_pane_uses_one_column() {
    let (fr, _) = field_layout(Rect::new(0, 0, 40, 24));
    assert_eq!(fr[0].x, fr[2].x);
    assert!(fr[1].y > fr[0].y && fr[2].y > fr[1].y);
}

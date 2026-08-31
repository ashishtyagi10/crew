use super::*;

#[test]
fn clock_section_has_rule_and_centered_time() {
    let cells = clock_cells("14:03:09", "Sat 21 Jun", 24);
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
    assert!(clock_cells("12:00:00", "Mon", 6).is_empty());
}

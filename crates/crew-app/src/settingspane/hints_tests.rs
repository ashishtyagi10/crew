//! The scroll arrows stand in the margin, not on a card's border.
use super::*;
use crate::config::CrewConfig;

#[test]
fn the_scroll_arrows_sit_outside_the_cards() {
    let _g = crate::app::theme_test_guard();
    let p = SettingsPane::new(CrewConfig::default(), Vec::new());
    let (cols, rows) = (56u16, 30u16);
    let cells = p.cells(cols, rows);
    let down: Vec<_> = cells.iter().filter(|c| c.c == '\u{2193}').collect();
    assert_eq!(down.len(), 1, "a short pane says there is more below");
    let d = down[0];
    assert_eq!(d.col, cols - 1, "in the last column, the margin");
    // The card border column beside it is still border (or blank), never
    // the arrow itself.
    let beside = cells.iter().find(|c| c.row == d.row && c.col == cols - 2);
    assert!(beside.is_none_or(|c| c.c != '\u{2193}'));
}

//! The `⇡N` on the LOG rule is the scroll actually applied, not the offset
//! the wheel asked for. `log_back` is clamped against the widest window the
//! nav can ever give the LOG, while the drawn window is usually smaller, so
//! the rule said `⇡7` over a tail that was following live.
use super::{log_cells, LogEntry, LogLevel};

fn entries(n: usize) -> Vec<LogEntry> {
    (0..n)
        .map(|i| LogEntry {
            level: LogLevel::Info,
            text: format!("line{i}"),
        })
        .collect()
}

/// Row 0 as drawn: the last cell written to a column wins, as on screen.
fn rule(cells: &[crew_render::CellView]) -> String {
    let mut by_col = std::collections::BTreeMap::new();
    for c in cells.iter().filter(|c| c.row == 0) {
        by_col.insert(c.col, c.c);
    }
    by_col.values().collect()
}

#[test]
fn the_mark_is_the_scroll_applied() {
    let _g = crate::app::theme_test_guard();
    let e = entries(10);
    // Eight rows show eight of ten: at most two back, whatever was asked.
    assert!(rule(&log_cells(&e, 30, 8, 7)).contains("\u{21e1}2"));
    assert!(rule(&log_cells(&e, 30, 8, 1)).contains("\u{21e1}1"));
    // Everything fits: no scroll is possible, so no mark however far back.
    assert!(!rule(&log_cells(&e, 30, 10, 5)).contains('\u{21e1}'));
    assert!(!rule(&log_cells(&e, 30, 8, 0)).contains('\u{21e1}'));
}

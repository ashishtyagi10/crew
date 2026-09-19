use super::*;
use crate::applog::{LogEntry, LogLevel};
use crate::navlog::log_cells;
use crew_render::CellView;

fn info(s: &str) -> LogEntry {
    LogEntry {
        level: LogLevel::Info,
        text: s.to_string(),
    }
}

/// The drawn row `r`, left to right.
fn row_text(cells: &[CellView], r: u16) -> String {
    let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == r).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect()
}

/// The cut: a docked nav can afford its clock, the collapsed rail cannot.
#[test]
fn the_clock_is_kept_where_a_message_still_reads() {
    assert!(!stamps_fit(18));
    assert!(!stamps_fit(22));
    assert!(stamps_fit(23));
    assert!(stamps_fit(27));
}

/// A stamp is recognised by SHAPE, since it is a prefix of the buffered line
/// rather than a field of it.
#[test]
fn a_stamp_is_found_by_its_shape() {
    assert_eq!(split_stamp("23:12 one"), ("23:12 ", "one"));
    assert_eq!(split_stamp("not stamped"), ("", "not stamped"));
    assert_eq!(split_stamp("2:12 one"), ("", "2:12 one"));
}

/// On the collapsed rail the clock was taking a third of the section and
/// every line read `crew v0…`. The stamp is the value that drops, and the
/// message takes the six columns it was using.
#[test]
fn a_narrow_log_drops_the_clock_and_keeps_the_words() {
    let _g = crate::app::theme_test_guard();
    let e = [info("23:12 restored 3 panes")];
    let narrow = row_text(&log_cells(&e, 18, 3, 0), 1);
    assert!(!narrow.contains("23:12"), "{narrow:?}");
    assert!(narrow.contains("restored 3"), "{narrow:?}");
    let docked = row_text(&log_cells(&e, 28, 3, 0), 1);
    assert!(docked.contains("23:12"), "{docked:?}");
}

/// Dropped, not blanked: six spaces where the clock was would give the
/// message nothing.

#[test]
fn the_dropped_clock_leaves_no_gap() {
    let _g = crate::app::theme_test_guard();
    let e = [info("23:12 one"), info("23:12 two")];
    let cells = log_cells(&e, 18, 3, 0);
    for row in 1..=2 {
        let first = cells
            .iter()
            .filter(|c| c.row == row)
            .map(|c| c.col)
            .min()
            .expect("a drawn row");
        assert_eq!(first, 2, "row {row} starts past the text column");
    }
}

/// The section keeps its shape either way: nothing escapes the card, at any
/// width the resize edge allows.

#[test]
fn no_line_escapes_the_card_at_any_width() {
    let _g = crate::app::theme_test_guard();
    let e = [info("23:12 a message that is much too long for a rail")];
    for cols in 4..48u16 {
        for c in log_cells(&e, cols, 3, 0) {
            assert!(c.col < cols, "{cols}: a cell at {} escaped", c.col);
        }
    }
}

//! The selected row's wash keeps air at both ends of its words: the time
//! touched the wash's right edge while the names began three columns in
//! (2026-10-30 shot survey), so a selected row looked pushed to the right.
use super::tests::{draw, thread};

#[test]
fn the_time_ends_two_columns_short_of_the_wash_on_every_density() {
    let _g = crate::app::theme_test_guard();
    for (cols, two) in [(60, true), (60, false), (120, false), (34, true)] {
        let c = draw(&thread(), cols, two, true);
        let top: Vec<_> = c.iter().filter(|c| c.row == 0).collect();
        assert_eq!(top.len(), usize::from(cols), "{cols}: the wash is whole");
        let last = top.iter().filter(|c| c.c != ' ').map(|c| c.col).max();
        assert_eq!(last, Some(cols - 3), "{cols} two={two}: the time's end");
        let first = top.iter().filter(|c| c.c != ' ').map(|c| c.col).min();
        assert_eq!(first, Some(1), "{cols}: the unread dot, then the names");
    }
}

use std::time::Duration;

use crate::model::{GridSize, TermCore};

fn text(core: &TermCore) -> String {
    let mut cs = core.cells(true);
    cs.sort_by_key(|c| (c.row, c.col));
    cs.iter().map(|c| c.c).collect()
}

/// The program opened an update and died before closing it: what it wrote
/// shows once the update's time is up, not never.
#[test]
fn an_update_left_open_is_drawn_once_its_time_is_up() {
    let mut core = TermCore::new(GridSize { cols: 20, rows: 3 });
    core.feed(b"\x1b[?2026hframe one");
    assert!(
        !text(&core).contains("frame"),
        "held while the update is open"
    );
    assert!(!core.flush_stale_sync(), "not before its deadline");
    std::thread::sleep(Duration::from_millis(200));
    assert!(core.flush_stale_sync(), "ended past it");
    assert!(
        text(&core).contains("frameone"),
        "blanks are not cells: {}",
        text(&core)
    );
    assert!(!core.flush_stale_sync(), "once");
}

/// An update the program closes itself never needs it.
#[test]
fn a_closed_update_is_left_alone() {
    let mut core = TermCore::new(GridSize { cols: 20, rows: 3 });
    core.feed(b"\x1b[?2026hok\x1b[?2026l");
    assert!(text(&core).contains("ok"));
    std::thread::sleep(Duration::from_millis(200));
    assert!(!core.flush_stale_sync());
}

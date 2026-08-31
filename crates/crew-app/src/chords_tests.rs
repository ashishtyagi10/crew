use super::{broadcast_label, next_active_index, swap_target};

#[test]
fn broadcast_label_reflects_state() {
    assert_eq!(broadcast_label(true), "broadcast: all panes");
    assert_eq!(broadcast_label(false), "broadcast: off");
}

#[test]
fn next_active_wraps_and_skips() {
    let active = [false, true, false, true];
    assert_eq!(next_active_index(&active, 0), Some(1));
    assert_eq!(next_active_index(&active, 1), Some(3));
    assert_eq!(next_active_index(&active, 3), Some(1)); // wraps past the end
    assert_eq!(next_active_index(&[false, false], 0), None);
    assert_eq!(next_active_index(&[], 0), None);
}

#[test]
fn swap_target_bounds() {
    assert_eq!(swap_target(0, 1, 1), None); // single pane
    assert_eq!(swap_target(0, 3, -1), None); // already leftmost
    assert_eq!(swap_target(1, 3, -1), Some(0));
    assert_eq!(swap_target(2, 3, 1), None); // already rightmost
    assert_eq!(swap_target(1, 3, 1), Some(2));
}

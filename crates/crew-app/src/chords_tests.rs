use super::{broadcast_label, next_active_index};

#[test]
fn broadcast_label_reflects_state() {
    assert_eq!(
        broadcast_label(true),
        "broadcast: typing goes to every terminal \u{2014} /broadcast turns it off"
    );
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

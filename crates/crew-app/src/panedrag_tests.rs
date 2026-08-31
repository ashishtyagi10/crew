use super::{is_drop_target, past_threshold, publish_drop, swap_for, CardDrag};

fn carried(pane: usize, moved: bool) -> CardDrag {
    CardDrag {
        pane,
        from: (0.0, 0.0),
        moved,
    }
}

#[test]
fn a_card_dropped_on_another_swaps_the_two() {
    assert_eq!(swap_for(carried(0, true), Some(3), 4), Some((0, 3)));
}

#[test]
fn a_press_that_never_travelled_swaps_nothing() {
    // Otherwise every click on a legend would reorder the grid.
    assert_eq!(swap_for(carried(0, false), Some(3), 4), None);
}

#[test]
fn a_card_dropped_on_itself_or_on_nothing_stays_put() {
    assert_eq!(swap_for(carried(2, true), Some(2), 4), None, "on itself");
    assert_eq!(swap_for(carried(2, true), None, 4), None, "off the grid");
}

/// A pane can close mid-drag (its process exits): the release must not
/// index past the end of a vec that shrank under it.
#[test]
fn a_pane_that_closed_mid_drag_is_not_swapped() {
    assert_eq!(swap_for(carried(0, true), Some(5), 2), None, "target gone");
    assert_eq!(swap_for(carried(5, true), Some(0), 2), None, "source gone");
}

#[test]
fn a_drag_needs_real_travel_before_it_counts() {
    let from = (100.0, 100.0);
    assert!(!past_threshold(from, (102.0, 101.0)), "a hand shake is not");
    assert!(past_threshold(from, (100.0, 120.0)), "20px down is");
    assert!(past_threshold(from, (80.0, 100.0)), "and so is 20px back");
}

/// The only test that touches the shared atomic, so it can never race
/// another one in this binary. Covers both what it publishes and what it
/// reports: exactly one card lights, the unnumbered lone card never does
/// (a swap needs two), and a repeat publish is not a change.
#[test]
fn one_card_lights_at_a_time_and_republishing_is_not_a_change() {
    publish_drop(0);
    assert!(publish_drop(2), "picking a target is a change");
    assert!(is_drop_target(2));
    assert!(!is_drop_target(1), "and only that one");
    assert!(!is_drop_target(0), "never the lone card");
    assert!(!publish_drop(2), "still on it");
    assert!(publish_drop(0), "leaving it is a change");
    assert!(!is_drop_target(2), "dropping clears it");
}

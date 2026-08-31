use super::{next_run, DOUBLE_CLICK, MAX_RUN};
use std::time::Instant;

#[test]
fn a_first_click_starts_the_run() {
    assert_eq!(next_run(None, Instant::now(), 0), 1);
}

#[test]
fn clicks_in_quick_succession_widen_the_run() {
    let t = Instant::now();
    assert_eq!(next_run(Some((t, 0, 1)), t, 0), 2, "word");
    assert_eq!(next_run(Some((t, 0, 2)), t, 0), 3, "line");
}

#[test]
fn the_run_caps_and_starts_over_rather_than_sticking() {
    // A hand resting on the button must not leave the widest gesture
    // latched: the fourth click is a fresh single click.
    let t = Instant::now();
    assert_eq!(next_run(Some((t, 0, MAX_RUN)), t, 0), 1);
}

#[test]
fn a_click_on_another_pane_starts_its_own_run() {
    let t = Instant::now();
    assert_eq!(next_run(Some((t, 0, 1)), t, 1), 1);
}

#[test]
fn a_slow_second_click_is_a_new_single_click() {
    let then = Instant::now() - DOUBLE_CLICK - std::time::Duration::from_millis(1);
    assert_eq!(next_run(Some((then, 0, 1)), Instant::now(), 0), 1);
}

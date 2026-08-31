use super::*;
use crate::motion::MotionLevel;

fn counter() -> Counter {
    crate::motion::set_level(MotionLevel::Full);
    Counter::default()
}

/// A number seen for the first time is simply shown.
#[test]
fn first_sight_is_settled() {
    let _g = crate::app::motion_test_guard();
    let c = counter();
    assert_eq!(c.tick(100.0, 1_000), 100.0);
    assert!(!c.live(1_000));
}

#[test]
fn a_changed_value_sweeps_to_it() {
    let _g = crate::app::motion_test_guard();
    let c = counter();
    let now = 1_000;
    assert_eq!(c.tick(100.0, now), 100.0);
    assert_eq!(c.tick(200.0, now), 100.0, "sweep starts where it was");
    let mid = c.tick(200.0, now + COUNT_MS / 2);
    assert!(mid > 100.0 && mid < 200.0, "mid was {mid}");
    assert_eq!(c.tick(200.0, now + COUNT_MS), 200.0);
}

/// The point of the module: a counter that arrives stops asking for frames.
#[test]
fn a_reached_value_settles() {
    let _g = crate::app::motion_test_guard();
    let c = counter();
    c.tick(42.0, 0);
    c.tick(99.0, 0);
    assert!(c.live(1));
    c.tick(99.0, COUNT_MS);
    assert!(!c.live(COUNT_MS), "an arrived counter must go quiet");
}

/// A second change mid-sweep continues from where the display had got to;
/// restarting from the old *target* would visibly jump backwards.
#[test]
fn a_change_mid_sweep_continues_from_the_shown_value() {
    let _g = crate::app::motion_test_guard();
    let c = counter();
    c.tick(0.0, 0);
    c.tick(100.0, 0);
    let shown = c.tick(100.0, COUNT_MS / 2);
    let after = c.tick(200.0, COUNT_MS / 2);
    assert!(
        (after - shown).abs() < 1e-9,
        "display jumped from {shown} to {after}"
    );
}

/// An unchanged target must not restart the sweep — a value read every
/// frame would otherwise animate forever and the app would never idle.
#[test]
fn an_unchanged_target_does_not_restart() {
    let _g = crate::app::motion_test_guard();
    let c = counter();
    c.tick(7.0, 0);
    for f in 1..40 {
        c.tick(7.0, f * 20);
    }
    assert!(!c.live(COUNT_MS + 1), "a steady value kept the app awake");
}

#[test]
fn motion_off_shows_the_true_value_immediately() {
    let _g = crate::app::motion_test_guard();
    let c = counter();
    crate::motion::set_level(MotionLevel::Off);
    c.tick(55.0, 900);
    assert_eq!(c.tick(70.0, 900), 70.0, "no sweep at off");
    assert!(!c.live(900));
    crate::motion::set_level(MotionLevel::Full);
}

/// Two panes animate their own numbers. The global registry this replaced
/// would have had them share one counter and overwrite each other.
#[test]
fn counters_are_per_surface() {
    let _g = crate::app::motion_test_guard();
    let (a, b) = (Readouts::default(), Readouts::default());
    a.cost.tick(10.0, 0);
    a.cost.tick(20.0, 0);
    b.cost.tick(500.0, 0);
    assert_eq!(b.cost.tick(500.0, 0), 500.0, "b must not see a's sweep");
    assert!(a.any_live(1) && !b.any_live(1));
}

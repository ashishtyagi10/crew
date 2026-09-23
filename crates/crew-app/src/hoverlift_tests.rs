use super::*;

/// The hovered card rises to its share of a lift, and once there asks for no
/// more frames.
#[test]
fn a_hovered_card_rises_and_settles() {
    let mut h = HoverLift::default();
    assert!(h.aim(Some(2)));
    assert!(h.moving());
    h.step(16, MotionLevel::Full);
    let part = h.lift(2);
    assert!(part > 0.0 && part < HOVER, "part of the way: {part}");
    h.step(10_000, MotionLevel::Full);
    assert_eq!(h.lift(2), HOVER);
    assert!(!h.moving());
    assert!(!h.aim(Some(2)), "the same card again is no change");
}

/// Moving to another card hands the lift over: the new one rises while the
/// old one settles, and the old one is forgotten once it is down.
#[test]
fn the_lift_hands_over_and_the_old_card_is_forgotten() {
    let mut h = HoverLift::default();
    h.aim(Some(0));
    h.step(10_000, MotionLevel::Full);
    h.aim(Some(1));
    h.step(16, MotionLevel::Full);
    assert!(h.lift(0) > 0.0 && h.lift(0) < HOVER, "still settling");
    assert!(h.lift(1) > 0.0, "already rising");
    h.step(10_000, MotionLevel::Full);
    assert_eq!((h.lift(0), h.lift(1)), (0.0, HOVER));
    assert_eq!(h.lifts.len(), 1, "a card back on the page is dropped");
}

/// Pointer gone: everything settles and nothing is left moving.
#[test]
fn nothing_hovered_settles_everything() {
    let mut h = HoverLift::default();
    h.aim(Some(3));
    h.step(10_000, MotionLevel::Full);
    h.aim(None);
    h.step(10_000, MotionLevel::Off);
    assert_eq!(h.lift(3), 0.0);
    assert!(!h.moving() && h.lifts.is_empty());
}

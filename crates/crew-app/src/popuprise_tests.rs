use super::{Rise, RISE_MS};
use crate::motion::MotionLevel::{Full, Off};

/// The frame a pop-up opens on starts the rise: half a row low, then up.
#[test]
fn opening_starts_a_rise_that_settles() {
    let mut r = Rise::default();
    assert_eq!(r.drop_rows(1_000), 0.0, "nothing open, nothing to rise");
    r.tick_at(false, 1_000, Full);
    r.tick_at(true, 1_000, Full);
    assert!(r.live(1_000));
    assert_eq!(r.drop_rows(1_000), 0.5);
    let mid = r.drop_rows(1_000 + RISE_MS / 2);
    assert!(mid > 0.0 && mid < 0.5, "{mid}");
    assert!(!r.live(1_000 + RISE_MS));
    assert_eq!(r.drop_rows(1_000 + RISE_MS), 0.0);
}

/// Staying open does not restart it; closing and reopening replays it.
#[test]
fn staying_open_keeps_the_rise_and_reopening_replays_it() {
    let mut r = Rise::default();
    r.tick_at(true, 1_000, Full);
    r.tick_at(true, 1_000 + 100, Full);
    assert!(r.drop_rows(1_000 + 100) < 0.5, "not restarted");
    r.tick_at(false, 2_000, Full);
    assert_eq!(r.drop_rows(2_000), 0.0);
    r.tick_at(true, 3_000, Full);
    assert_eq!(r.drop_rows(3_000), 0.5, "replayed");
}

/// With motion off there is no rise at all: the card is in place at once.
#[test]
fn motion_off_seats_the_card_at_once() {
    let mut r = Rise::default();
    r.tick_at(true, 1_000, Off);
    assert!(!r.live(1_000));
    assert_eq!(r.drop_rows(1_000), 0.0);
}

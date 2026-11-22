use super::*;
use crate::motion::MotionLevel::{Full, Off, Subtle};

const RESTING: u64 = TTL_MS / 2;
const LEAVING: u64 = TTL_MS - EXIT_MS / 2;

/// Motion off: a resting card, a card in what would be its exit window and a
/// card sliding in ask for nothing; a card past its time asks for the one
/// frame that removes it.
#[test]
fn motion_off_asks_only_for_the_removal_frame() {
    assert!(!wants_frame(RESTING, false, Off));
    assert!(!wants_frame(LEAVING, false, Off), "no dissolve to draw");
    assert!(!wants_frame(10, true, Off));
    assert!(
        wants_frame(TTL_MS, false, Off),
        "the frame that takes it away"
    );
    assert!(wants_frame(TTL_MS + 500, false, Off));
}

/// With motion: in and out are drawn, the rest is not; a card past its time
/// has already slid off the edge, so it asks for nothing.
#[test]
fn motion_draws_the_slide_in_and_the_exit_window() {
    for level in [Full, Subtle] {
        assert!(wants_frame(10, true, level));
        assert!(!wants_frame(RESTING, false, level));
        assert!(wants_frame(LEAVING, false, level));
        assert!(!wants_frame(TTL_MS, false, level));
    }
}

/// Motion off holds the card whole and in place to the end; with motion it
/// is half dissolved and a quarter of the way out halfway through the exit.
#[test]
fn leaving_holds_still_at_motion_off() {
    assert_eq!(leave(LEAVING, 400.0, Off), (0.0, 0.0));
    assert_eq!(leave(TTL_MS - 1, 400.0, Off), (0.0, 0.0));
    let (x, fade) = leave(LEAVING, 400.0, Full);
    assert!((fade - 0.5).abs() < 1e-3, "{fade}");
    assert!((x - 100.0).abs() < 0.5, "{x}");
    assert_eq!(leave(RESTING, 400.0, Full), (0.0, 0.0));
}

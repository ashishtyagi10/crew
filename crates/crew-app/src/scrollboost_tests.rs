use super::*;

#[test]
fn the_first_tick_of_a_gesture_is_unscaled() {
    let mut b = Boost::default();
    assert_eq!(b.apply(3, 1_000), 3);
}

/// Flicking moves pages, nudging moves lines, and the two are the same wheel.
#[test]
fn ticks_that_keep_coming_scroll_further() {
    let mut b = Boost::default();
    let first = b.apply(3, 1_000);
    let second = b.apply(3, 1_040);
    let third = b.apply(3, 1_080);
    assert!(second > first, "{second} !> {first}");
    assert!(third > second, "{third} !> {second}");
}

/// A pause is the whole signal: a scroll you resume after stopping starts
/// slow again, or "start slow" means nothing for reading.
#[test]
fn a_pause_resets_the_gesture() {
    let mut b = Boost::default();
    for t in 0..6 {
        b.apply(3, 1_000 + t * 40);
    }
    let after_pause = b.apply(3, 5_000);
    assert_eq!(after_pause, 3, "the gesture survived a four-second pause");
}

/// A single flick may cross a long log and must not cross the whole
/// scrollback: the multiplier is capped.
#[test]
fn the_multiplier_is_capped() {
    let mut b = Boost::default();
    let mut last = 0;
    for t in 0..200 {
        last = b.apply(3, 1_000 + t * 20);
    }
    assert_eq!(last, (3.0 * MAX) as i32, "{last}");
}

/// Direction survives, and a real tick never rounds away to nothing.
#[test]
fn direction_and_the_smallest_tick_are_preserved() {
    let mut b = Boost::default();
    assert_eq!(b.apply(-3, 1_000), -3);
    assert!(b.apply(-3, 1_040) < -3);
    let mut c = Boost::default();
    assert_eq!(c.apply(1, 1_000), 1);
    assert_eq!(c.apply(0, 1_040), 0, "no tick, no scroll");
}

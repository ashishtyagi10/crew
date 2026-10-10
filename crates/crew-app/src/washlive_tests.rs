//! The wash clock's flywheel: a change of pace is a glide.
use super::*;

/// The flywheel: a pane that starts working spins the page up to its busy
/// pace over a couple of seconds instead of lurching to four times the speed
/// in one frame, and when the work ends the page coasts down more slowly
/// still.
#[test]
fn a_change_of_pace_is_a_glide() {
    let (ambient, busy) = (Some(24_000), Some(6_000));
    let mut w = WashPhase::default();
    let mut t = 0;
    let step = |w: &mut WashPhase, t: &mut u64, pace| {
        let before = w.advance(*t, pace, MotionLevel::Full);
        *t += 100;
        let after = w.advance(*t, pace, MotionLevel::Full);
        (after - before).rem_euclid(1.0)
    };
    w.advance(t, ambient, MotionLevel::Full);
    for _ in 0..50 {
        step(&mut w, &mut t, ambient);
    }
    let (slow, fast) = (100.0 / 24_000.0, 100.0 / 6_000.0);
    assert!(
        (step(&mut w, &mut t, ambient) - slow).abs() < 1e-5,
        "steady at the ambient pace"
    );
    // The first busy frame barely quickens...
    let first = step(&mut w, &mut t, busy);
    assert!(first < slow * 1.5, "a lurch: {first} vs {slow} a frame");
    // ...and five seconds in, the page turns at the busy pace.
    let mut last = first;
    for _ in 0..50 {
        let s = step(&mut w, &mut t, busy);
        assert!(s >= last - 1e-6, "spin-up must not falter: {s} < {last}");
        last = s;
    }
    assert!(
        (last - fast).abs() < fast * 0.02,
        "at the busy pace: {last} vs {fast}"
    );
    // When the work ends it coasts: a second later still well above idle,
    // and slower to come down than it was to go up.
    for _ in 0..10 {
        last = step(&mut w, &mut t, ambient);
    }
    assert!(last > slow * 2.0, "it should coast down, not stop: {last}");
}

use super::*;

/// A process that has never drifted draws the still page — every resting shot
/// in the tree is calibrated against it — and an idle crew that never
/// advances stays asleep.
#[test]
fn a_page_that_never_drifted_is_asleep() {
    let mut w = WashPhase::default();
    assert_eq!(w.live(), 0.0);
    for t in [0, 500, 10_000] {
        w.advance(t, None, MotionLevel::Full);
    }
    assert_eq!(w.live(), 0.0, "held frames must not wake the page");
}

/// The flow swells in over WAKE_MS of drift — eased, so it starts slow — and
/// then stays fully awake through every wrap of both clocks: it never stops
/// on a clock the way the old once-a-revolution exhale did.
#[test]
fn the_page_wakes_over_a_few_seconds_and_stays_awake() {
    let mut w = WashPhase::default();
    let pace = Some(24_000);
    w.advance(0, pace, MotionLevel::Full);
    let mut prev = 0.0;
    let mut t = 0;
    while t < WAKE_MS as u64 {
        t += 100;
        w.advance(t, pace, MotionLevel::Full);
        assert!(
            w.live() >= prev,
            "the wake must only rise: {} < {prev}",
            w.live()
        );
        prev = w.live();
    }
    assert_eq!(w.live(), 1.0, "awake after WAKE_MS of drift");
    // Eased: a tenth of the way in, it is well under a tenth on.
    let mut early = WashPhase::default();
    early.advance(0, pace, MotionLevel::Full);
    early.advance(WAKE_MS as u64 / 10, pace, MotionLevel::Full);
    assert!(early.live() < 0.05, "eased start, got {}", early.live());
    // Two minutes of drift (several turns of both clocks) never dims it.
    while t < 120_000 {
        t += 100;
        w.advance(t, pace, MotionLevel::Full);
        assert_eq!(w.live(), 1.0, "the flow dipped at {t} ms");
    }
}

/// A hold (focus lost, Motion off) freezes the flow where it is rather than
/// putting the page back to sleep — the frozen frame is the last one drawn.
#[test]
fn a_hold_keeps_the_page_awake() {
    let mut w = WashPhase::default();
    w.advance(0, Some(6_000), MotionLevel::Full);
    for t in (100..=4_000).step_by(100) {
        w.advance(t, Some(6_000), MotionLevel::Full);
    }
    w.advance(5_000, None, MotionLevel::Full);
    w.advance(6_000, Some(6_000), MotionLevel::Off);
    assert_eq!(w.live(), 1.0);
}

/// The eddies keep their own time: when the orbit and the slow clock have
/// both come round to where they started (48 s at a 24 s pace), the eddies
/// have not, so the page as a whole does not replay.
#[test]
fn the_eddies_never_fall_into_step() {
    let mut w = WashPhase::default();
    let pace = Some(24_000);
    w.advance(0, pace, MotionLevel::Full);
    for t in (100..=48_000).step_by(100) {
        w.advance(t, pace, MotionLevel::Full);
    }
    let c = w.clocks();
    let off = |x: f32| x.min(1.0 - x);
    assert!(
        off(c.phase) < 1e-3 && off(c.wander) < 1e-3,
        "both round: {c:?}"
    );
    assert!(off(c.eddy) > 0.1, "the eddies came round with them: {c:?}");
    assert_eq!(c.live, 1.0);
}

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
        let before = w.clocks().phase;
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

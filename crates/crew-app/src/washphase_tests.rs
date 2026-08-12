use super::*;

/// One revolution per `drift_ms`, measured off the busy frames themselves.
/// The FIRST busy frame only stamps the clock (there is no previous frame to
/// measure from), so the turn is counted from the second one on.
#[test]
fn busy_frames_turn_once_per_drift_period() {
    let mut w = WashPhase::default();
    assert_eq!(
        w.advance(0, true, 1_000, MotionLevel::Full),
        0.0,
        "first frame only stamps"
    );
    let mut prev = 0.0;
    for step in 1..10 {
        let p = w.advance(step * 100, true, 1_000, MotionLevel::Full);
        assert!(p > prev, "phase must climb: {p} !> {prev} at step {step}");
        assert!((p - step as f32 / 10.0).abs() < 1e-4, "step {step}: {p}");
        prev = p;
    }
    // A full period later it is back where it started, not at 1.0 or beyond.
    let p = w.advance(1_000, true, 1_000, MotionLevel::Full);
    assert!(p < 1e-4, "a full turn must wrap to zero, got {p}");
}

/// An idle crew never repaints, so the wash must HOLD rather than track the
/// wall clock — and the quiet time must not be paid back in one lurch when
/// activity resumes.
#[test]
fn quiet_time_holds_the_phase_and_is_never_paid_back() {
    let mut w = WashPhase::default();
    w.advance(0, true, 1_000, MotionLevel::Full);
    assert!((w.advance(100, true, 1_000, MotionLevel::Full) - 0.1).abs() < 1e-4);
    // Idle frames (a settings repaint, a resize) hold it exactly.
    for t in [200, 900, 5_000] {
        assert!(
            (w.advance(t, false, 1_000, MotionLevel::Full) - 0.1).abs() < 1e-6,
            "idle frame at {t} moved the wash"
        );
    }
    // Activity resumes a minute later: the first frame back stamps only.
    assert!(
        (w.advance(60_000, true, 1_000, MotionLevel::Full) - 0.1).abs() < 1e-6,
        "lurch"
    );
    assert!(
        (w.advance(60_100, true, 1_000, MotionLevel::Full) - 0.2).abs() < 1e-4,
        "resumed"
    );
}

/// A stalled frame (slow build, lid closed, a blocking read on the winit
/// thread) contributes at most `MAX_STEP_MS` — enough to keep moving, not
/// enough to teleport the pools.
#[test]
fn a_stalled_frame_is_clamped() {
    let mut w = WashPhase::default();
    w.advance(0, true, 1_000, MotionLevel::Full);
    let p = w.advance(5_000, true, 1_000, MotionLevel::Full);
    assert!(
        (p - MAX_STEP_MS as f32 / 1_000.0).abs() < 1e-4,
        "a 5s gap must clamp to {MAX_STEP_MS}ms, got phase {p}"
    );
}

/// Motion off is a genuine off: the aurora is frozen, so those frames stay
/// byte-identical too.
#[test]
fn motion_off_freezes_the_wash() {
    let mut w = WashPhase::default();
    w.advance(0, true, 1_000, MotionLevel::Off);
    let p = w.advance(500, true, 1_000, MotionLevel::Off);
    assert_eq!(p, 0.0, "motion off must not drift");
    // Subtle is not off: the wash still turns there, just as the ring does.
    assert!(w.advance(1_000, true, 1_000, MotionLevel::Subtle) == 0.0);
    assert!(w.advance(1_100, true, 1_000, MotionLevel::Subtle) > 0.0);
}

/// The phase is a fraction of a turn forever — no unbounded float growing
/// past the precision where small deltas stop registering.
#[test]
fn phase_stays_inside_one_turn() {
    let mut w = WashPhase::default();
    for i in 0..1_000u64 {
        let p = w.advance(i * 60, true, 6_000, MotionLevel::Full);
        assert!((0.0..1.0).contains(&p), "frame {i}: phase {p} out of range");
    }
}

/// A theme with no drift period can't divide the delta by it — hold instead
/// of handing the shader a NaN.
#[test]
fn a_zero_drift_period_holds() {
    let mut w = WashPhase::default();
    w.advance(0, true, 0, MotionLevel::Full);
    assert_eq!(w.advance(1_000, true, 0, MotionLevel::Full), 0.0);
}

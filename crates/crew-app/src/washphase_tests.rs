use super::*;

/// One revolution per `drift_ms`, measured off the busy frames themselves.
/// The FIRST busy frame only stamps the clock (there is no previous frame to
/// measure from), so the turn is counted from the second one on.
#[test]
fn busy_frames_turn_once_per_drift_period() {
    let mut w = WashPhase::default();
    assert_eq!(
        w.advance(0, Some(1_000), MotionLevel::Full),
        0.0,
        "first frame only stamps"
    );
    let mut prev = 0.0;
    for step in 1..10 {
        let p = w.advance(step * 100, Some(1_000), MotionLevel::Full);
        assert!(p > prev, "phase must climb: {p} !> {prev} at step {step}");
        assert!((p - step as f32 / 10.0).abs() < 1e-4, "step {step}: {p}");
        prev = p;
    }
    // A full period later it is back where it started, not at 1.0 or beyond.
    let p = w.advance(1_000, Some(1_000), MotionLevel::Full);
    assert!(p < 1e-4, "a full turn must wrap to zero, got {p}");
}

/// An idle crew never repaints, so the wash must HOLD rather than track the
/// wall clock — and the quiet time must not be paid back in one lurch when
/// activity resumes.
#[test]
fn quiet_time_holds_the_phase_and_is_never_paid_back() {
    let mut w = WashPhase::default();
    w.advance(0, Some(1_000), MotionLevel::Full);
    assert!((w.advance(100, Some(1_000), MotionLevel::Full) - 0.1).abs() < 1e-4);
    // Idle frames (a settings repaint, a resize) hold it exactly.
    for t in [200, 900, 5_000] {
        assert!(
            (w.advance(t, None, MotionLevel::Full) - 0.1).abs() < 1e-6,
            "idle frame at {t} moved the wash"
        );
    }
    // Activity resumes a minute later: the first frame back stamps only.
    assert!(
        (w.advance(60_000, Some(1_000), MotionLevel::Full) - 0.1).abs() < 1e-6,
        "lurch"
    );
    assert!(
        (w.advance(60_100, Some(1_000), MotionLevel::Full) - 0.2).abs() < 1e-4,
        "resumed"
    );
}

/// A stalled frame (slow build, lid closed, a blocking read on the winit
/// thread) contributes at most `MAX_STEP_MS` — enough to keep moving, not
/// enough to teleport the pools.
#[test]
fn a_stalled_frame_is_clamped() {
    let mut w = WashPhase::default();
    w.advance(0, Some(1_000), MotionLevel::Full);
    let p = w.advance(5_000, Some(1_000), MotionLevel::Full);
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
    w.advance(0, Some(1_000), MotionLevel::Off);
    let p = w.advance(500, Some(1_000), MotionLevel::Off);
    assert_eq!(p, 0.0, "motion off must not drift");
    // Subtle is not off: the wash still turns there, just as the ring does.
    assert!(w.advance(1_000, Some(1_000), MotionLevel::Subtle) == 0.0);
    assert!(w.advance(1_100, Some(1_000), MotionLevel::Subtle) > 0.0);
}

/// The phase is a fraction of a turn forever — no unbounded float growing
/// past the precision where small deltas stop registering.
#[test]
fn phase_stays_inside_one_turn() {
    let mut w = WashPhase::default();
    for i in 0..1_000u64 {
        let p = w.advance(i * 60, Some(6_000), MotionLevel::Full);
        assert!((0.0..1.0).contains(&p), "frame {i}: phase {p} out of range");
    }
}

/// A theme with no drift period can't divide the delta by it — hold instead
/// of handing the shader a NaN.
#[test]
fn a_zero_drift_period_holds() {
    let mut w = WashPhase::default();
    w.advance(0, Some(0), MotionLevel::Full);
    assert_eq!(w.advance(1_000, Some(0), MotionLevel::Full), 0.0);
}

/// The busy pace is the theme's own; ambient is [`AMBIENT_MULT`] times slower;
/// with neither, the wash holds. Busy wins when both are true, so a working
/// pane never has its wash slowed down by the idle setting.
#[test]
fn the_pace_is_busy_then_ambient_then_still() {
    assert_eq!(pace(6_000, true, false), Some(6_000), "busy");
    assert_eq!(
        pace(6_000, true, true),
        Some(6_000),
        "busy outranks ambient"
    );
    assert_eq!(pace(6_000, false, true), Some(90_000), "ambient");
    assert_eq!(pace(6_000, false, false), None, "still");
}

/// A theme with no gradient to move never asks for a pace, ambient or not —
/// turning a phase nothing reads would buy frames for no pixels.
#[test]
fn a_theme_with_no_drift_period_never_moves() {
    for (busy, ambient) in [(true, true), (true, false), (false, true), (false, false)] {
        assert_eq!(
            pace(0, busy, ambient),
            None,
            "busy={busy} ambient={ambient}"
        );
    }
}

/// Ambient really is slower, not just different — the whole point is that
/// idle motion is a texture rather than a signal.
#[test]
fn ambient_is_slower_than_busy_by_a_wide_margin() {
    let busy = pace(6_000, true, false).unwrap();
    let ambient = pace(6_000, false, true).unwrap();
    assert!(
        ambient >= busy * 10,
        "ambient {ambient}ms is not far enough from busy {busy}ms"
    );
}

/// Crossing from ambient to busy and back must not jump the pools across the
/// page: both paces accumulate onto the same phase, and a faster pace only
/// changes how much each frame adds.
#[test]
fn changing_pace_mid_drift_is_continuous() {
    let mut w = WashPhase::default();
    let slow = pace(1_000, false, true); // 15_000ms per revolution
    w.advance(0, slow, MotionLevel::Full);
    let before = w.advance(100, slow, MotionLevel::Full);
    // The very next frame is busy: it may speed up, but must not teleport.
    let after = w.advance(200, pace(1_000, true, false), MotionLevel::Full);
    assert!(after > before, "it keeps going forward");
    assert!(
        after - before < 0.2,
        "a pace change jumped the phase by {}",
        after - before
    );
}

/// Motion off is a genuine off at either pace.
#[test]
fn motion_off_holds_the_ambient_drift_too() {
    let mut w = WashPhase::default();
    w.advance(0, pace(1_000, false, true), MotionLevel::Full);
    let moved = w.advance(500, pace(1_000, false, true), MotionLevel::Full);
    assert!(moved > 0.0, "premise: it was drifting");
    for t in [600, 5_000, 60_000] {
        assert_eq!(
            w.advance(t, pace(1_000, false, true), MotionLevel::Off),
            moved,
            "Motion off must hold the wash at {t}"
        );
    }
}

/// The four fences on the ambient drift, each checked by turning exactly one
/// of them off. This is the only motion in crew that repaints an otherwise
/// idle window, so every one of them is load-bearing.
#[test]
fn each_fence_alone_stops_the_ambient_drift() {
    let _g = crate::app::theme_test_guard();
    let mut app = crate::app::CrewApp::default();
    app.config.ambient_drift = true;
    app.win_focus = None;
    assert!(app.ambient_drift(), "premise: all four fences pass");

    app.config.ambient_drift = false;
    assert!(!app.ambient_drift(), "the setting");
    app.config.ambient_drift = true;

    app.win_focus = Some(false);
    assert!(!app.ambient_drift(), "another window has the OS focus");
    app.win_focus = Some(true);
    assert!(app.ambient_drift(), "and it comes back when focus returns");

    crate::motion::set_level(MotionLevel::Off);
    assert!(!app.ambient_drift(), "Motion off");
    crate::motion::set_level(MotionLevel::Full);
    assert!(app.ambient_drift());
}

/// A window that has never been told about focus is treated as focused: not
/// every platform sends `Focused(true)` for a window that opens focused, and
/// a drift that never starts is worse than one that runs while hidden.
#[test]
fn a_window_that_was_never_told_counts_as_focused() {
    let _g = crate::app::theme_test_guard();
    let app = crate::app::CrewApp::default();
    assert_eq!(app.win_focus, None, "premise: nothing has said either way");
    assert!(app.ambient_drift());
}

/// The ambient drift is deliberately NOT a term in `wants_animation_frame`:
/// that predicate keeps meaning "some transient animation is in flight", and
/// this — the one thing that repaints an otherwise idle window — is its own
/// branch with its own, much coarser throttle. An idle app must therefore say
/// no to the first and yes to the second.
#[test]
fn an_idle_app_wants_no_animation_frame_but_does_want_the_drift() {
    let _g = crate::app::theme_test_guard();
    let mut app = crate::app::CrewApp::default();
    app.config.ambient_drift = true;
    let now = crate::anim::now_ms();
    assert!(
        !app.wants_animation_frame(now),
        "premise: nothing transient is animating"
    );
    assert!(app.ambient_drift(), "but the wash is still turning");
    // …and with the setting off, an idle app asks for nothing at all, which
    // is exactly the behaviour crew had before this existed.
    app.config.ambient_drift = false;
    assert!(!app.wants_animation_frame(now) && !app.ambient_drift());
}

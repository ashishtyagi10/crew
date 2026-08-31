use super::*;

#[test]
fn the_curve_spans_zero_to_one_and_clamps() {
    assert!(out_cubic(0.0).abs() < 1e-6, "must start at 0");
    assert!((out_cubic(1.0) - 1.0).abs() < 1e-5, "must end at 1");
    assert!(out_cubic(-5.0).abs() < 1e-6);
    assert!((out_cubic(5.0) - 1.0).abs() < 1e-5);
}

/// Ease-out is ahead of linear in its first half — that is what "fast
/// departure" means, and it is the whole reason to use it.
#[test]
fn out_cubic_leads_linear() {
    assert!(out_cubic(0.25) > 0.25);
    assert!(out_cubic(0.5) > 0.5);
}

#[test]
fn timeline_runs_then_settles() {
    let t = Timeline::start(1_000, 300, MotionLevel::Full);
    assert_eq!(t.progress(1_000), 0.0);
    assert!((t.progress(1_150) - 0.5).abs() < 1e-6);
    assert_eq!(t.progress(1_300), 1.0);
    assert!(t.live(1_299));
    assert!(
        !t.live(1_300),
        "a settled timeline must stop scheduling frames"
    );
}

/// The reduce-motion contract: Off doesn't animate faster, it doesn't
/// animate at all — one frame at the final state, then silence.
#[test]
fn motion_off_settles_instantly() {
    let t = Timeline::start(1_000, 300, MotionLevel::Off);
    assert_eq!(t.progress(1_000), 1.0);
    assert!(!t.live(1_000));
}

#[test]
fn subtle_is_quicker_than_full() {
    let now = 500;
    let subtle = Timeline::start(now, 400, MotionLevel::Subtle);
    let full = Timeline::start(now, 400, MotionLevel::Full);
    assert!(subtle.progress(now + 200) > full.progress(now + 200));
}

/// Progress is clamped, so a timeline read long after it ended reports its
/// final state rather than running away.
/// The `Default` timeline is the resting state, not a pending one.
#[test]
fn default_is_already_settled() {
    let t = Timeline::default();
    assert_eq!(t.progress(0), 1.0);
    assert_eq!(t.progress(u64::MAX), 1.0);
    assert!(!t.live(0));
}

#[test]
fn progress_never_exceeds_one() {
    let t = Timeline::start(0, 100, MotionLevel::Full);
    assert_eq!(t.progress(10_000), 1.0);
    assert!((t.eased(10_000, out_cubic) - 1.0).abs() < 1e-6);
}

/// Clock reads before the start (possible when a timeline is stamped from
/// a slightly later `now`) must not underflow into a huge progress value.
#[test]
fn progress_before_start_is_zero() {
    let t = Timeline::start(1_000, 100, MotionLevel::Full);
    assert_eq!(t.progress(0), 0.0);
}

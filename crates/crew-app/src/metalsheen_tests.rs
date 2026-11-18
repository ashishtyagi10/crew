use super::*;

#[test]
fn the_light_rests_off_metal_and_at_motion_off() {
    let rest = REST;
    assert_eq!(sweep(4_000, false, MotionLevel::Full), rest);
    assert_eq!(sweep(4_000, true, MotionLevel::Off), rest);
    assert!(!moving(true, MotionLevel::Off, true));
    assert!(!moving(false, MotionLevel::Full, true));
    assert!(
        !moving(true, MotionLevel::Full, false),
        "an unfocused window costs no frames"
    );
    assert!(moving(true, MotionLevel::Subtle, true));
}

#[test]
fn the_sheen_sways_across_the_window() {
    let quarter = sweep(DRIFT_MS / 4, true, MotionLevel::Full)[0];
    let three = sweep(DRIFT_MS * 3 / 4, true, MotionLevel::Full)[0];
    assert!((quarter - DRIFT).abs() < 1e-3, "{quarter}");
    assert!((three + DRIFT).abs() < 1e-3, "{three}");
    let subtle = sweep(DRIFT_MS / 4, true, MotionLevel::Subtle)[0];
    assert!((subtle - DRIFT * 0.5).abs() < 1e-3, "{subtle}");
}

#[test]
fn a_glint_crosses_now_and_then() {
    let start = sweep(0, true, MotionLevel::Full);
    let mid = sweep(GLINT_RUN_MS / 2, true, MotionLevel::Full);
    let after = sweep(GLINT_RUN_MS + 10, true, MotionLevel::Full);
    assert!(
        start[2] < 0.01 && start[1] < 0.0,
        "fades in off the left: {start:?}"
    );
    assert!(mid[2] > 0.99 && (mid[1] - 0.5).abs() < 0.01, "{mid:?}");
    assert_eq!(after[2], 0.0);
    assert_eq!(sweep(GLINT_RUN_MS / 2, true, MotionLevel::Subtle)[2], 0.0);
}

#[test]
fn a_glint_draws_every_tick_only_while_it_runs() {
    let f = MotionLevel::Full;
    assert!(glinting(GLINT_RUN_MS / 2, true, f, true));
    assert!(!glinting(GLINT_RUN_MS + 10, true, f, true));
    assert!(!glinting(GLINT_RUN_MS / 2, true, f, false));
    assert!(!glinting(GLINT_RUN_MS / 2, true, MotionLevel::Subtle, true));
}

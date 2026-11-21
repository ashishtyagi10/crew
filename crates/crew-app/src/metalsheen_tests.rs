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

#[test]
fn the_pool_runs_on_its_own_clock() {
    let f = MotionLevel::Full;
    assert_eq!(REST[3], 0.0, "at rest the pool lies still");
    assert!((sweep(4_300, true, f)[3] - 4.3).abs() < 1e-4);
    assert!(
        (sweep(LIQUID_WRAP_MS + 500, true, f)[3] - 0.5).abs() < 1e-4,
        "wraps"
    );
    assert!(sweep(0, true, f)[3] > 0.0, "drops fall even at the wrap");
    assert!(
        sweep(4_300, true, MotionLevel::Subtle)[3] < 0.0,
        "subtle swells but drops nothing"
    );
}

/// The shader's wrap is this clock's, and every swell turns whole times in
/// it, so the pool never hops.
#[test]
fn the_shader_wraps_the_pool_with_the_clock() {
    let src = include_str!("../../crew-render/src/glass.wgsl");
    let at = src.find("const LIQUID_WRAP: f32 = ").expect("LIQUID_WRAP");
    let rest = &src[at..];
    let v: f32 = rest[rest.find('=').unwrap() + 1..rest.find(';').unwrap()]
        .trim()
        .parse()
        .unwrap();
    assert_eq!(v * 1000.0, LIQUID_WRAP_MS as f32);
    let period = |n: &str| -> f32 {
        let at = src.find(&format!("const {n}: ")).expect(n);
        let rest = &src[at..];
        rest[rest.find('=').unwrap() + 1..rest.find(';').unwrap()]
            .trim()
            .parse()
            .unwrap()
    };
    let cycle = period("LIQUID_DROP_EVERY") * period("LIQUID_DROPS");
    assert_eq!(v % cycle, 0.0, "the drops' cycle divides the wrap");
}

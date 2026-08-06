use super::*;

fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

#[test]
fn step_moves_a_time_scaled_fraction_toward_the_target() {
    let (out, settled) = step(
        r(0.0, 0.0, 100.0, 100.0),
        r(200.0, 0.0, 100.0, 100.0),
        90,
        false,
    );
    // dt == TAU → k = 1 - 1/e ≈ 0.632: x covers ~63% of the 200px gap.
    assert!(!settled);
    assert!(
        (out.x - 126.4).abs() < 0.5,
        "x moved {} of 200, wanted ~126",
        out.x
    );
    assert_eq!(out.y, 0.0);
}

#[test]
fn repeated_steps_converge_and_snap_exactly() {
    let target = r(300.0, 200.0, 400.0, 240.0);
    let mut cur = r(0.0, 0.0, 100.0, 100.0);
    let mut frames = 0;
    loop {
        let (out, settled) = step(cur, target, 50, false);
        cur = out;
        frames += 1;
        if settled {
            break;
        }
        assert!(frames < 60, "smoothing must converge, stuck at {cur:?}");
    }
    // The snap makes settling exact, not merely close.
    assert_eq!((cur.x, cur.y, cur.w, cur.h), (300.0, 200.0, 400.0, 240.0));
    assert!(
        frames > 3,
        "a real glide takes several frames, got {frames}"
    );
}

#[test]
fn snap_flag_teleports_in_one_frame() {
    let (out, settled) = step(
        r(0.0, 0.0, 100.0, 100.0),
        r(500.0, 0.0, 80.0, 80.0),
        50,
        true,
    );
    assert!(settled);
    assert_eq!(out, r(500.0, 0.0, 80.0, 80.0));
}

#[test]
fn fresh_spawn_with_zero_rect_snaps() {
    // A new pane has no prior rect; the assemble animation owns entrances.
    let (out, settled) = step(
        r(0.0, 0.0, 0.0, 0.0),
        r(10.0, 10.0, 100.0, 100.0),
        50,
        false,
    );
    assert!(settled);
    assert_eq!(out.w, 100.0);
}

#[test]
fn already_at_target_settles_immediately() {
    let t = r(10.0, 10.0, 100.0, 100.0);
    let (out, settled) = step(t, t, 50, false);
    assert!(settled);
    assert_eq!(out, t);
}

#[test]
fn frame_dt_clamps_idle_stretches() {
    assert_eq!(frame_dt(1_000, 950), 50);
    // 10s idle → clamped, so the next glide still animates instead of
    // covering the whole distance in its first frame.
    assert_eq!(frame_dt(11_000, 1_000), 100);
    assert_eq!(frame_dt(100, 200), 0, "clock going backwards is a no-op");
}

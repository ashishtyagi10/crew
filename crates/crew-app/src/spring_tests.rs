use super::*;

/// The frequency the grid runs at, so the numbers below are the ones crew
/// actually feels.
const OMEGA: f32 = 18.0;

/// Step a spring in 16ms frames until it settles, or give up. Returns the ms
/// it took, and the furthest it ever got PAST the target — measured in the
/// direction it was travelling, so the same helper answers for a spring
/// coming back down as for one going up.
fn run(mut s: Spring, target: f32) -> (u64, f32) {
    let sign = (target - s.pos).signum();
    let mut t = 0;
    let mut overshoot: f32 = 0.0;
    while !s.settled(target) && t < 5_000 {
        s.step(target, 16.0, OMEGA);
        t += 16;
        overshoot = overshoot.max((s.pos - target) * sign);
    }
    assert!(t < 5_000, "a spring that never settles repaints forever");
    (t, overshoot)
}

/// The invariant the whole render loop is built on: every animation ends.
/// A spring that keeps twitching below the snap threshold is an app that
/// never goes idle.
#[test]
fn a_spring_arrives_and_stops() {
    let (ms, _) = run(Spring::at(0.0), 400.0);
    assert!((120..900).contains(&ms), "settled in {ms}ms");
}

/// Critically damped means it does not go past. A pane bouncing off its own
/// tile would be reading as playful about a layout change the user asked for.
#[test]
fn it_does_not_overshoot_the_target() {
    let (_, over) = run(Spring::at(0.0), 400.0);
    assert!(over < 1.0, "overshot by {over}px");
    // …from either side.
    let (_, over) = run(Spring::at(400.0), 0.0);
    assert!(over < 1.0, "overshot by {over}px coming back");
}

/// The entire reason to prefer a spring over smoothing: it remembers it was
/// moving. Retargeted mid-flight, it must carry its velocity through instead
/// of restarting from rest — which is what makes a second grid change while
/// the first is still reflowing read as one continuous motion.
#[test]
fn it_carries_velocity_through_a_retarget() {
    let mut s = Spring::at(0.0);
    for _ in 0..6 {
        s.step(400.0, 16.0, OMEGA);
    }
    let moving = s.vel;
    assert!(
        moving > 50.0,
        "the fixture must actually be in flight: {moving}"
    );

    // Retarget. One frame later it must still be travelling in the same
    // direction at a comparable speed — a from-rest restart would show up
    // here as a velocity near zero.
    s.step(600.0, 16.0, OMEGA);
    assert!(
        s.vel > moving * 0.5,
        "velocity collapsed on retarget: {moving} -> {}",
        s.vel
    );
}

/// A long frame (the first after an idle stretch) must not blow the
/// integrator up. Substepping is what keeps semi-implicit Euler stable, and
/// without it a big `dt` sends the position to infinity — visible as a pane
/// vanishing off the canvas.
#[test]
fn a_long_frame_is_substepped_rather_than_trusted() {
    let mut s = Spring::at(0.0);
    for _ in 0..40 {
        s.step(400.0, 100.0, OMEGA);
        assert!(
            s.pos.is_finite() && s.pos.abs() < 2_000.0,
            "blew up: {}",
            s.pos
        );
    }
    assert!(s.settled(400.0), "still at {} moving {}", s.pos, s.vel);
}

/// Position alone is not arrival. A spring crossing its target at speed is
/// mid-flight, and calling that settled snaps it dead — the exact teleport
/// the animation exists to remove.
#[test]
fn settling_needs_both_position_and_stillness() {
    let on_target_but_flying = Spring {
        pos: 400.0,
        vel: 300.0,
    };
    assert!(!on_target_but_flying.settled(400.0));
    let stopped_but_short = Spring {
        pos: 380.0,
        vel: 0.0,
    };
    assert!(!stopped_but_short.settled(400.0));
    assert!(Spring::at(400.0).settled(400.0));
}

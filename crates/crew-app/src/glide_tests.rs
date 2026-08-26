use super::*;

fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

/// A glide converges and parks EXACTLY on its target — a resting layout is
/// the placed layout, not a fraction of a pixel off it, because everything
/// downstream (cell math, hit rects) reads the drawn rect.
#[test]
fn repeated_steps_converge_and_snap_exactly() {
    let target = r(300.0, 200.0, 400.0, 240.0);
    let mut g = Glide::at(r(0.0, 0.0, 100.0, 100.0));
    let mut frames = 0;
    loop {
        let (_, settled) = g.step(target, 16, false);
        frames += 1;
        if settled {
            break;
        }
        assert!(
            frames < 200,
            "a glide must converge, stuck at {:?}",
            g.rect()
        );
    }
    let cur = g.rect();
    assert_eq!((cur.x, cur.y, cur.w, cur.h), (300.0, 200.0, 400.0, 240.0));
    // …in about the quarter-second the smoothing it replaced took.
    let ms = frames * 16;
    assert!((150..600).contains(&ms), "settled in {ms}ms");
}

/// Every edge is its own spring: a pane moving right while growing taller is
/// two different distances at two different speeds, and one shared scalar
/// would have to be wrong about one of them.
#[test]
fn each_edge_travels_on_its_own() {
    let mut g = Glide::at(r(0.0, 0.0, 100.0, 100.0));
    let (out, settled) = g.step(r(200.0, 0.0, 100.0, 100.0), 16, false);
    assert!(!settled);
    assert!(out.x > 0.0, "x must have moved: {}", out.x);
    assert_eq!((out.y, out.w, out.h), (0.0, 100.0, 100.0), "the rest held");
}

/// Motion off is a genuine off everywhere in crew: the final state draws once
/// and nothing reschedules.
#[test]
fn motion_off_lands_on_the_target_in_one_frame() {
    let target = r(10.0, 20.0, 30.0, 40.0);
    let mut g = Glide::at(r(0.0, 0.0, 100.0, 100.0));
    let (out, settled) = g.step(target, 16, true);
    assert!(settled);
    assert_eq!((out.x, out.y, out.w, out.h), (10.0, 20.0, 30.0, 40.0));
    assert_eq!(
        g.rect(),
        target,
        "and the spring adopts it, with no velocity"
    );
}

/// A pane with no prior rect is arriving, not reflowing — the assemble
/// animation owns entrances, and springing a card out of the origin would
/// fight it.
#[test]
fn a_pane_with_no_prior_rect_snaps() {
    let target = r(10.0, 20.0, 300.0, 400.0);
    let mut g = Glide::at(r(0.0, 0.0, 0.0, 0.0));
    let (out, settled) = g.step(target, 16, false);
    assert!(settled && out == target);
}

/// The reason for the whole rewrite. A grid that changes again mid-reflow has
/// to continue the motion, not restart it: with the old exponential smoothing
/// a retarget was indistinguishable from a fresh start at rest, so closing two
/// panes in quick succession read as two unrelated animations.
#[test]
fn a_retarget_mid_flight_keeps_its_velocity() {
    let mut g = Glide::at(r(0.0, 0.0, 100.0, 100.0));
    for _ in 0..6 {
        g.step(r(400.0, 0.0, 100.0, 100.0), 16, false);
    }
    let travelled = g.rect().x;
    assert!(
        travelled > 5.0,
        "the fixture must be in flight: {travelled}"
    );

    // Same distance again from here, once as a retarget and once from rest.
    let mut fresh = Glide::at(g.rect());
    let far = r(travelled + 300.0, 0.0, 100.0, 100.0);
    let moving = g.step(far, 16, false).0.x - travelled;
    let started = fresh.step(far, 16, false).0.x - travelled;
    assert!(
        moving > started * 1.5,
        "a moving pane must cover more ground than one starting from rest \
         ({moving} vs {started}) — velocity was dropped on retarget"
    );
}

/// Something outside the glide can move a pane (zoom's own lerp, a drag, a
/// resize). The spring has to adopt that, or it integrates from a stale
/// position and yanks the pane back toward where it thought it was.
#[test]
fn reseeding_adopts_a_position_the_spring_did_not_produce() {
    let mut g = Glide::at(r(0.0, 0.0, 100.0, 100.0));
    for _ in 0..6 {
        g.step(r(400.0, 0.0, 100.0, 100.0), 16, false);
    }
    g.reseed(r(50.0, 60.0, 100.0, 100.0));
    assert_eq!(g.rect(), r(50.0, 60.0, 100.0, 100.0));
    // With the velocity killed, one frame toward the SAME place it was
    // already heading moves it exactly as far as a spring starting at rest.
    let mut fresh = Glide::at(r(50.0, 60.0, 100.0, 100.0));
    let target = r(400.0, 60.0, 100.0, 100.0);
    assert_eq!(g.step(target, 16, false).0, fresh.step(target, 16, false).0);
}

#[test]
fn frame_dt_clamps_a_stale_frame() {
    assert_eq!(frame_dt(1_000, 984), 16);
    assert_eq!(frame_dt(9_000, 1_000), 100, "an idle stretch is clamped");
    assert_eq!(frame_dt(100, 900), 0, "a clock that went backwards");
}

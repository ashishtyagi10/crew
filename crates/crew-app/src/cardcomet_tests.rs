//! The indeterminate comet moves with Motion on and holds a level, centred
//! segment with Motion off.
use super::SWEEP_MS;
use crate::cardpaint::card_paint;
use crate::motion::{set_level, MotionLevel};
use crate::panecard::Bar;

fn busy() -> Bar<'static> {
    Bar {
        progress: Some(crew_term::Progress {
            percent: None,
            alarm: false,
        }),
        ..Default::default()
    }
}

/// Two instants a quarter-bounce apart, drawn at `level`.
fn two_frames(level: MotionLevel) -> (Vec<crew_render::Paint>, Vec<crew_render::Paint>) {
    set_level(level);
    let at = |now| card_paint(40, 10, &busy(), (8.0, 16.0), now);
    (at(0), at(SWEEP_MS / 4))
}

/// The ink left and right of the drawn segment's middle.
fn halves(ps: &[crew_render::Paint]) -> (f32, f32) {
    let lo = ps.iter().map(|p| p.x).fold(f32::MAX, f32::min);
    let hi = ps.iter().map(|p| p.x + p.w).fold(f32::MIN, f32::max);
    let mid = (lo + hi) / 2.0;
    let ink = |p: &&crew_render::Paint| p.w * p.h * p.alpha;
    let left = ps
        .iter()
        .filter(|p| p.x + p.w <= mid)
        .map(|p| ink(&p))
        .sum();
    let right = ps.iter().filter(|p| p.x >= mid).map(|p| ink(&p)).sum();
    (left, right)
}

#[test]
fn the_comet_sweeps_with_motion_on() {
    let _g = crate::app::theme_test_guard();
    let (a, b) = two_frames(MotionLevel::Full);
    assert_ne!(a, b, "a moving comet draws a different frame later");
    let (l, r) = halves(&a);
    assert!((l - r).abs() > 0.2 * (l + r), "a moving comet has a tail");
}

#[test]
fn motion_off_holds_the_comet_still_level_and_centred() {
    let _g = crate::app::theme_test_guard();
    let (a, b) = two_frames(MotionLevel::Off);
    assert_eq!(a, b, "motion off: the same frame at every instant");
    assert!(
        !a.is_empty(),
        "still drawn: the card still says it is working"
    );
    // Level: no tail, so the ink either side of its middle is the same.
    let (l, r) = halves(&a);
    assert!((l - r).abs() < 0.02 * (l + r), "{l} vs {r}");
    // Centred between the two corners (cols 1..39).
    let lo = a.iter().map(|p| p.x).fold(f32::MAX, f32::min);
    let hi = a.iter().map(|p| p.x + p.w).fold(f32::MIN, f32::max);
    assert!(((lo + hi) / 2.0 - 20.0).abs() < 0.5, "{lo}..{hi}");
}

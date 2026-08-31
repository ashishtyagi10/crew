use super::*;
use crate::motion::MotionLevel;

fn rect() -> Rect {
    Rect {
        x: 400.0,
        y: 50.0,
        w: 300.0,
        h: 200.0,
    }
}

fn ghost(exit: Exit, now: u64) -> Ghost {
    crate::motion::set_level(MotionLevel::Full);
    Ghost::new(rect(), "build".into(), exit, now)
}

#[test]
fn collapse_runs_from_whole_to_nothing() {
    let _g = crate::app::motion_test_guard();
    let g = ghost(Exit::Closed, 1_000);
    assert!((g.collapse_t(1_000) - 1.0).abs() < 1e-6, "starts whole");
    assert_eq!(g.collapse_t(1_000 + COLLAPSE_MS), 0.0, "ends gone");
    assert!(g.collapse_t(1_150) < 1.0);
}

#[test]
fn a_closed_card_stays_where_it_was() {
    let _g = crate::app::motion_test_guard();
    let g = ghost(Exit::Closed, 0);
    assert_eq!(g.rect_at(0).x, 400.0);
    assert_eq!(g.rect_at(150).x, 400.0);
}

/// Minimize means "it went into the nav", so the card has to travel that
/// way — a card that retracted in place would say "closed" instead.
#[test]
fn a_minimized_card_travels_toward_the_nav() {
    let _g = crate::app::motion_test_guard();
    let g = ghost(Exit::Minimized, 0);
    assert_eq!(g.rect_at(0).x, 400.0);
    let mid = g.rect_at(150).x;
    assert!(mid < 400.0 && mid > 0.0, "mid-flight x was {mid}");
    assert!(
        g.rect_at(COLLAPSE_MS).x.abs() < 1e-3,
        "should reach the nav"
    );
}

#[test]
fn prune_drops_only_finished_ghosts() {
    let _g = crate::app::motion_test_guard();
    let mut gs = vec![ghost(Exit::Closed, 0), ghost(Exit::Closed, 10_000)];
    prune(&mut gs, 1_000);
    assert_eq!(gs.len(), 1, "the settled ghost should be gone");
}

/// Reduce-motion: no collapse, no scheduled frames, no ghost.
#[test]
fn motion_off_leaves_nothing_behind() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(MotionLevel::Off);
    let g = Ghost::new(rect(), "build".into(), Exit::Closed, 5_000);
    assert!(!g.live(5_000));
    let mut gs = vec![g];
    prune(&mut gs, 5_000);
    assert!(gs.is_empty(), "a dismissed pane must simply vanish at off");
    crate::motion::set_level(MotionLevel::Full);
}

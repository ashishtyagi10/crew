//! What the caret's wake has to be: behind the cursor, never ahead of it,
//! gone within a keystroke, and absent entirely when there was no move to
//! trace.
use super::*;

/// A wake mid-fade, from `a` to `b`, read `ms` after the move — as the one
/// rectangle it covers, since most of what has to be true of it is true of the
/// whole streak rather than of any one of its slices.
fn wake(a: (u16, u16), b: (u16, u16), ms: u64) -> Option<Paint> {
    let mut t = Trail::default();
    t.observe_at(Some(a), 1_000, motion::MotionLevel::Full);
    t.observe_at(Some(b), 1_000, motion::MotionLevel::Full);
    hull(&t.paint(1_000 + ms, (200, 200, 200)))
}

/// The bounding box of a sliced streak, carrying the brightest slice's alpha.
fn hull(ps: &[Paint]) -> Option<Paint> {
    let first = *ps.first()?;
    Some(ps.iter().fold(first, |a, p| Paint {
        x: a.x.min(p.x),
        y: a.y.min(p.y),
        w: (a.x + a.w).max(p.x + p.w) - a.x.min(p.x),
        h: (a.y + a.h).max(p.y + p.h) - a.y.min(p.y),
        color: a.color,
        alpha: a.alpha.max(p.alpha),
    }))
}

#[test]
fn a_caret_that_has_not_moved_leaves_nothing() {
    let mut t = Trail::default();
    t.observe_at(Some((4, 2)), 1_000, motion::MotionLevel::Full);
    t.observe_at(Some((4, 2)), 1_040, motion::MotionLevel::Full);
    assert!(t.paint(1_040, (200, 200, 200)).is_empty());
    assert!(!t.live(1_040), "a still caret must not ask for frames");
}

/// The first cursor of a pane's life has come from nowhere: there is no ground
/// between "not on screen" and where it appeared, and drawing a bar from the
/// origin would put a streak across the first line of every new shell.
#[test]
fn a_caret_that_just_appeared_has_no_wake() {
    let mut t = Trail::default();
    t.observe_at(Some((7, 3)), 1_000, motion::MotionLevel::Full);
    assert!(t.paint(1_000, (200, 200, 200)).is_empty());
    assert!(!t.live(1_000));
}

/// Typing: the streak covers the cell just left AND the cell arrived at, so
/// the two read as one object that moved.
#[test]
fn one_cell_right_drags_a_bar_from_the_cell_left_behind() {
    let p = wake((5, 2), (6, 2), 0).expect("a move draws a wake");
    assert_eq!(
        (p.x, p.w),
        (5.0, 2.0),
        "spans the departed cell and the head"
    );
    assert_eq!((p.y, p.h), (2.0, 1.0), "one row tall");
    assert!(
        p.alpha > 0.3 && p.alpha < 0.5,
        "a shadow, not a second cursor"
    );
}

/// A wake with two hard edges and one flat alpha is a *selection*; the ramp
/// into the caret is what makes the same rectangle read as speed.
#[test]
fn the_streak_is_brightest_at_the_caret_and_thins_behind_it() {
    let mut t = Trail::default();
    t.observe_at(Some((4, 0)), 1_000, motion::MotionLevel::Full);
    t.observe_at(Some((16, 0)), 1_000, motion::MotionLevel::Full);
    let ps = t.paint(1_010, (200, 200, 200));
    assert!(ps.len() > 2, "a taper needs slices: {}", ps.len());
    let nearest = ps
        .iter()
        .max_by(|a, b| a.x.partial_cmp(&b.x).unwrap())
        .unwrap();
    let furthest = ps
        .iter()
        .min_by(|a, b| a.x.partial_cmp(&b.x).unwrap())
        .unwrap();
    assert!(
        nearest.alpha > furthest.alpha * 2.0,
        "the slice at the caret ({}) must dominate the one at the tail ({})",
        nearest.alpha,
        furthest.alpha
    );
}

/// The head is the cell the program says the caret is in. A wake that reached
/// past it would be a caret drawn in the wrong place.
#[test]
fn the_wake_never_reaches_past_the_caret() {
    for ms in [0, 30, 60, 100, 129] {
        let p = wake((9, 1), (4, 1), ms).expect("still fading");
        assert!(p.x >= 4.0, "at {ms}ms the wake started left of the head");
        assert!(
            p.x + p.w <= 10.0,
            "at {ms}ms the wake ran past the cell it came from"
        );
    }
}

#[test]
fn the_wake_shrinks_into_the_head_and_then_stops() {
    let first = wake((2, 0), (10, 0), 10).expect("live");
    let later = wake((2, 0), (10, 0), 90).expect("live");
    assert!(
        later.w < first.w,
        "the tail catches up: {} < {}",
        later.w,
        first.w
    );
    assert!(later.alpha < first.alpha, "and fades while it does");
    assert_eq!(wake((2, 0), (10, 0), WAKE_MS), None, "bounded");
}

/// A full-screen redraw moves the cursor a long way in one frame. A bar across
/// the whole pane would be louder than the caret it points at, so the far jump
/// leaves a mark on the cell it *left* instead.
#[test]
fn a_long_jump_ghosts_the_departed_cell_instead_of_barring_across() {
    let p = wake((0, 4), (90, 4), 10).expect("live");
    assert_eq!((p.x, p.w, p.y, p.h), (0.0, 1.0, 4.0, 1.0));
}

#[test]
fn a_jump_across_rows_ghosts_too() {
    let p = wake((3, 0), (3, 9), 10).expect("live");
    assert_eq!((p.x, p.w, p.y, p.h), (3.0, 1.0, 0.0, 1.0));
}

/// A newline is one row down and back to column zero — close enough to join,
/// and the one vertical move that happens constantly.
#[test]
fn a_newline_draws_a_wake_covering_both_rows() {
    let p = wake((11, 2), (0, 3), 0).expect("live");
    assert_eq!(
        (p.y, p.h),
        (2.0, 2.0),
        "covers the row left and the row landed on"
    );
    assert!(p.w > 1.0, "and the columns between");
}

/// The reduce-motion contract: Off is not a faster wake, it is no wake.
#[test]
fn motion_off_draws_nothing_and_schedules_nothing() {
    let mut t = Trail::default();
    t.observe_at(Some((1, 1)), 1_000, motion::MotionLevel::Off);
    t.observe_at(Some((2, 1)), 1_000, motion::MotionLevel::Off);
    assert!(t.paint(1_000, (200, 200, 200)).is_empty());
    assert!(!t.live(1_000));
}

/// Losing the caret (the program hiding it, or a scroll into history) ends the
/// wake rather than leaving one hanging over the page.
#[test]
fn a_caret_that_goes_away_takes_its_wake_with_it() {
    let mut t = Trail::default();
    t.observe_at(Some((5, 5)), 1_000, motion::MotionLevel::Full);
    t.observe_at(Some((6, 5)), 1_000, motion::MotionLevel::Full);
    assert!(!t.paint(1_010, (200, 200, 200)).is_empty());
    t.observe_at(None, 1_010, motion::MotionLevel::Full);
    assert!(t.paint(1_010, (200, 200, 200)).is_empty());
    assert!(!t.live(1_010));
}

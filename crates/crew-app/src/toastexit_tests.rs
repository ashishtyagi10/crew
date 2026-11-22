use super::*;
use crate::motion::MotionLevel::{Full, Off, Subtle};

const RESTING: u64 = TTL_MS / 2;
const LEAVING: u64 = TTL_MS - EXIT_MS / 2;

/// Motion off: no card asks for frames — not resting, not in what would be
/// its exit window, not past its time (that frame is the poll's prune's).
#[test]
fn motion_off_asks_for_no_frames() {
    for age in [10, RESTING, LEAVING, TTL_MS, TTL_MS + 500] {
        assert!(!wants_frame(age, age == 10, Off), "{age}");
    }
}

/// The poll's prune takes a card past its time away and says so once — the
/// one repaint — then has nothing more to say.
#[test]
fn expire_owes_one_repaint_then_nothing() {
    let mut t = crate::toast::Toasts::default();
    t.push("done".into(), "done", false, 1_000);
    assert!(!t.expire(1_000 + RESTING));
    assert!(t.expire(1_000 + TTL_MS), "the card goes: one repaint");
    assert!(!t.expire(1_000 + TTL_MS + 16), "and then quiet");
    assert_eq!(t.len(), 0);
}

/// With motion: in and out are drawn, the rest is not; a card past its time
/// has already slid off the edge, so it asks for nothing.
#[test]
fn motion_draws_the_slide_in_and_the_exit_window() {
    for level in [Full, Subtle] {
        assert!(wants_frame(10, true, level));
        assert!(!wants_frame(RESTING, false, level));
        assert!(wants_frame(LEAVING, false, level));
        assert!(!wants_frame(TTL_MS, false, level));
    }
}

/// Motion off holds the card whole and in place to the end; with motion it
/// is half dissolved and a quarter of the way out halfway through the exit.
#[test]
fn leaving_holds_still_at_motion_off() {
    assert_eq!(leave(LEAVING, 400.0, Off), (0.0, 0.0));
    assert_eq!(leave(TTL_MS - 1, 400.0, Off), (0.0, 0.0));
    let (x, fade) = leave(LEAVING, 400.0, Full);
    assert!((fade - 0.5).abs() < 1e-3, "{fade}");
    assert!((x - 100.0).abs() < 0.5, "{x}");
    assert_eq!(leave(RESTING, 400.0, Full), (0.0, 0.0));
}

/// The pointer resting on a card, and NO frames while it rests (a held stack asks for none):
/// neither the poll's prune nor the next drawn frame may time the card out.
#[test]
fn a_held_card_outlives_an_idle_gap() {
    use crate::layout::Rect;
    use crate::toast::push_toasts;
    let mut t = crate::toast::Toasts::default();
    t.push("waiting".into(), "waiting", true, 0);
    let content = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    let mut scenes = Vec::new();
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 100, None);
    let on = Some((scenes[0].x + 2.0, scenes[0].y + 2.0));
    scenes.clear();
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 200, on);
    let later = 200 + 3 * TTL_MS;
    assert!(!t.expire(later), "the poll leaves a held stack alone");
    scenes.clear();
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, later, on);
    assert_eq!(
        scenes.len(),
        1,
        "the next frame under the pointer still draws it"
    );
}

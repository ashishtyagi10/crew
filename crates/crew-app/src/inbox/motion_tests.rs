use super::*;
use crate::motion::{set_level, MotionLevel};

fn thread(id: &str, at_ms: i64) -> Thread {
    Thread {
        id: id.into(),
        at_ms,
        ..Thread::default()
    }
}

#[test]
fn each_motion_runs_its_course_and_motion_off_draws_only_the_end() {
    let _g = crate::app::motion_test_guard();
    let m = Motion {
        glide: Some((3, 1_000)),
        fold: Some(Fold {
            at: 2,
            thread: thread("x", 0),
            start: 1_000,
        }),
        fresh: Some((vec!["n".into()], 1_000)),
    };
    assert!(m.live(1_000));
    let (from, k) = m.glide(1_070).expect("halfway");
    assert!(from == 3 && k > 0.5 && k < 1.0, "eased: {k}");
    assert_eq!(m.glide(1_140), None, "settled");
    let lines = |now| m.folding(2, now).map(|(_, _, left, _)| left);
    assert_eq!(
        (lines(1_000), lines(1_150), lines(1_240)),
        (Some(2), Some(1), None)
    );
    let fade = m.folding(2, 1_120).map(|(.., f)| f).unwrap();
    assert!(fade > 0.4 && fade < 0.6, "{fade}");
    assert!(m.glow("n", 1_000).unwrap() > 0.99 && m.glow("o", 1_000).is_none());
    assert!(!m.live(2_300), "all bounded");
    let was = crate::motion::level();
    set_level(MotionLevel::Off);
    let settled = !m.live(1_000) && m.glide(1_000).is_none() && m.folding(2, 1_000).is_none();
    set_level(was);
    assert!(settled, "at Motion off nothing moves");
}

#[test]
fn new_mail_is_what_was_not_listed_or_has_mail_newer_than_it_had() {
    let old = [thread("a", 10), thread("b", 20)];
    let new = [thread("c", 30), thread("b", 25), thread("a", 10)];
    assert_eq!(fresh(&old, &new), ["c", "b"]);
    assert!(fresh(&[], &new).is_empty(), "the first list is not news");
}

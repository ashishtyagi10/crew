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

/// Fifty threads paged in, newest first, the selection on the forty-first.
fn deep() -> crate::inbox::InboxPane {
    use crate::inbox::worker::Worker;
    let (tx, _) = std::sync::mpsc::channel();
    let (_, rx) = std::sync::mpsc::channel();
    let mut p = crate::inbox::InboxPane::with(crate::inbox::Stage::Ready(Worker { tx, rx }));
    p.rows = (0..50)
        .map(|i| thread(&format!("t{i}"), 1_000 - i))
        .collect();
    p.sel = 40;
    p.more = true;
    p
}

/// A first page from Gmail: threads `t{i}` for each `i`, maybe a new one on top.
fn first(new: bool, ids: impl Iterator<Item = i64>, whole: bool) -> crate::inbox::worker::Got {
    let mut rows: Vec<Thread> = new.then(|| thread("new", 2_000)).into_iter().collect();
    rows.extend(ids.map(|i| thread(&format!("t{i}"), 1_000 - i)));
    crate::inbox::worker::Got::Page {
        query: String::new(),
        rows,
        next: Some("p3".into()),
        more: false,
        whole,
    }
}

/// The check after new mail (or after a thread filed here) lists the first
/// page again. The pages `j` brought in below it stay, and so does the place
/// in them: it used to cut the list back to the page and drop the selection
/// to its last row.
#[test]
fn a_fresh_first_page_keeps_the_pages_below_it_and_the_place() {
    let mut p = deep();
    p.take(first(true, 0..24, false));
    assert_eq!(p.rows.len(), 51);
    assert_eq!(
        (p.rows[0].id.as_str(), p.rows[p.sel].id.as_str()),
        ("new", "t40")
    );
    let mut ids: Vec<&str> = p.rows.iter().map(|t| t.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 51, "a thread listed twice");
}

/// A thread the fresh page no longer carries left the list (filed in
/// another client), and a page Gmail says is the whole list leaves nothing
/// below it.
#[test]
fn a_fresh_page_drops_what_it_no_longer_carries() {
    let mut p = deep();
    p.take(first(false, (0..26).filter(|&i| i != 3), false));
    assert!(!p.rows.iter().any(|t| t.id == "t3"));
    assert_eq!((p.rows.len(), p.rows[p.sel].id.as_str()), (49, "t40"));
    let mut whole = deep();
    whole.take(first(false, 0..10, true));
    assert_eq!(whole.rows.len(), 10);
}

/// A further page adds only the threads not already held: after a refresh
/// kept the rows below, Gmail's next page can begin with some of them.
#[test]
fn a_further_page_adds_only_the_threads_not_held() {
    let mut p = deep();
    p.take(crate::inbox::worker::Got::Page {
        query: String::new(),
        rows: (45..60)
            .map(|i| thread(&format!("t{i}"), 1_000 - i))
            .collect(),
        next: None,
        more: true,
        whole: true,
    });
    assert_eq!(p.rows.len(), 60);
    assert!(!p.more, "Gmail had nothing after it");
}

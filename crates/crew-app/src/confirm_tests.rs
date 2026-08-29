use super::*;

#[test]
fn the_first_run_asks_and_the_second_does_it() {
    let now = Instant::now();
    let mut p = Pending::default();
    assert!(!p.answered("closeall", now), "it went on the first run");
    assert!(
        p.answered("closeall", now),
        "the second run did not confirm"
    );
    // …and the confirmation is spent: a third run asks again.
    assert!(!p.answered("closeall", now));
}

/// A different command replaces the question rather than answering it — the
/// case this exists to catch is a second command arriving where you expected
/// your own answer.
#[test]
fn another_command_does_not_answer_the_first() {
    let now = Instant::now();
    let mut p = Pending::default();
    assert!(!p.answered("closeall", now));
    assert!(!p.answered("only", now), "one command confirmed another");
    // The pending one is now `only`, and `closeall` has to ask again.
    assert!(!p.answered("closeall", now));
    assert!(p.answered("closeall", now));
}

/// An answer that arrives long after the question is not an answer.
#[test]
fn a_stale_question_is_asked_again_rather_than_run() {
    let now = Instant::now();
    let mut p = Pending::default();
    assert!(!p.answered("closeall", now));
    assert!(!p.answered("closeall", now + WINDOW + Duration::from_secs(1)));
    // …and that re-ask is itself answerable.
    assert!(p.answered("closeall", now + WINDOW + Duration::from_secs(1)));
}

#[test]
fn clearing_forgets_the_question() {
    let now = Instant::now();
    let mut p = Pending::default();
    assert!(!p.answered("closeall", now));
    p.clear();
    assert!(
        !p.answered("closeall", now),
        "a cleared question was answered"
    );
}

/// The ask stood for ten seconds and was *visible* for three. For the other
/// seven, nothing on screen said that running the command again would close
/// every pane — and nothing said when the window had shut, either.
#[test]
fn the_question_stands_as_long_as_the_answer_does() {
    let mut p = Pending::default();
    let now = Instant::now();
    assert!(p.question(now).is_none(), "nothing asked, nothing shown");

    p.answered("closeall", now);
    p.asking("close all 4 panes? /closeall again");
    // Five seconds in — past the status flash, still inside the window.
    let mid = now + Duration::from_secs(5);
    assert_eq!(p.question(mid), Some("close all 4 panes? /closeall again"));
    // …and it is still answerable at that moment, which is the point.
    let mut q = Pending::default();
    q.answered("closeall", now);
    q.asking("x");
    assert!(q.answered("closeall", mid), "the window is still open");

    // Past the window: the bar stops saying it in the same instant the
    // second run stops meaning "yes".
    assert!(p.question(now + Duration::from_secs(11)).is_none());
}

/// Answering it takes the question off the bar.
#[test]
fn answering_clears_the_question() {
    let mut p = Pending::default();
    let now = Instant::now();
    p.answered("only", now);
    p.asking("close the other 3 panes? /only again");
    assert!(p.question(now).is_some());
    assert!(p.answered("only", now));
    assert!(p.question(now).is_none());
}

/// Whether anything is armed at all — what the dispatcher asks before running
/// something unrelated.
#[test]
fn armed_says_whether_a_second_press_would_do_anything() {
    let mut p = Pending::default();
    assert!(!p.armed());
    p.answered("closeall", Instant::now());
    assert!(p.armed());
    p.clear();
    assert!(!p.armed());
}

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

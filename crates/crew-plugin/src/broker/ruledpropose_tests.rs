//! The answer words, and that they are answers only while one is waiting.
use super::*;

use crate::broker::session::Session;

#[test]
fn only_the_exact_words_answer() {
    assert_eq!(word("keep"), Some(Word::Keep));
    assert_eq!(word("Keep it."), Some(Word::Keep));
    assert_eq!(word("forget"), Some(Word::Forget));
    assert_eq!(word("forget that!"), Some(Word::Forget));
    for task in [
        "yes",
        "no",
        "keep going",
        "keep the old parser",
        "forget correction 2",
        "forget about the tests",
    ] {
        assert_eq!(word(task), None, "{task:?}");
    }
}

#[test]
fn with_nothing_proposed_the_word_falls_through() {
    let s = Session::default();
    let mut said = Vec::new();
    let held = gate("keep", &s, &mut |e| {
        said.push(e);
        Ok(())
    });
    assert!(held.is_none(), "keep is a task when nothing waits");
    assert!(said.is_empty());
}

#[test]
fn the_proposal_line_names_it_and_both_words_once() {
    let s = Session::default();
    let line = propose(&s.recall, "don't touch Cargo.lock").expect("asked");
    assert!(line.contains("don't touch Cargo.lock"), "{line}");
    for w in ["\u{201c}keep\u{201d}", "\u{201c}forget\u{201d}"] {
        assert_eq!(line.matches(w).count(), 1, "{w} in {line}");
    }
    assert!(!line.contains('\n'), "one line: {line}");
    assert_eq!(propose(&s.recall, "don't touch Cargo.lock"), None);
    assert_eq!(
        answer(&s.recall, Word::Forget).as_deref(),
        Some("forgot: don't touch Cargo.lock")
    );
    assert_eq!(answer(&s.recall, Word::Keep), None);
}

/// "keep" typed while the correcting turn still runs is held back from its
/// tool rounds, so it reaches `gate` as a message; any other steer, and
/// "keep" with nothing waiting, still joins the run.
#[test]
fn a_word_for_a_waiting_correction_is_not_steered_into_the_run() {
    use crate::broker::steer::{take, tests as st};
    let _t = st::turn();
    let log = st::Log::default();
    let _task = st::taking(&log);
    let s = Session::default();
    steer(&s.recall, "crew".into(), "keep".into());
    assert_eq!(take(), ["keep"], "nothing waits: a steer like any other");
    propose(&s.recall, "don't touch Cargo.lock").expect("asked");
    steer(&s.recall, "crew".into(), "keep it".into());
    steer(&s.recall, "crew".into(), "keep going".into());
    assert_eq!(take(), ["keep going"]);
    assert_eq!(
        log.lock().unwrap().len(),
        2,
        "the held word is never announced"
    );
    assert_eq!(
        recall::lock(&s.recall).proposed(),
        Some("don't touch Cargo.lock")
    );
}

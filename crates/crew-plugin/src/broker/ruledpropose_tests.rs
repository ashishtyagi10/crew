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
    assert!(!line.contains('\n'), "one line: {line}");
    assert_eq!(propose(&s.recall, "don't touch Cargo.lock"), None);
    assert_eq!(
        answer(&s.recall, Word::Forget).as_deref(),
        Some("forgot: don't touch Cargo.lock")
    );
    assert_eq!(answer(&s.recall, Word::Keep), None);
}

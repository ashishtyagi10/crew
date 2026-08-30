//! Undo has to give back what a person thinks they did — a word, not a
//! letter, and never a road they have already left.
use super::*;

fn typed(at: u32, s: &str) -> Change {
    Change {
        at,
        removed: String::new(),
        inserted: s.into(),
        caret: at,
    }
}

fn deleted(at: u32, s: &str) -> Change {
    Change {
        at,
        removed: s.into(),
        inserted: String::new(),
        caret: at + s.len() as u32,
    }
}

/// Undoing a sentence one letter at a time is the same as having no undo.
#[test]
fn a_run_of_typing_is_taken_back_as_one() {
    let mut h = History::default();
    for (i, c) in "word".chars().enumerate() {
        h.record(typed(10 + i as u32, &c.to_string()));
    }
    let back = h.undo().expect("something to undo");
    assert_eq!(back.removed, "word", "the whole word goes at once");
    assert_eq!(back.at, 10);
    assert!(h.undo().is_none(), "and there was only one run");
}

/// A space closes the run, so undo gives back a word at a time rather than a
/// paragraph.
#[test]
fn a_space_ends_the_run() {
    let mut h = History::default();
    for (i, c) in "two words".chars().enumerate() {
        h.record(typed(i as u32, &c.to_string()));
    }
    assert_eq!(h.undo().expect("a run").removed, "words");
    assert_eq!(h.undo().expect("another").removed, "two ");
}

#[test]
fn a_newline_ends_the_run_too() {
    let mut h = History::default();
    h.record(typed(0, "a"));
    h.record(typed(1, "\n"));
    h.record(typed(2, "b"));
    assert_eq!(h.undo().expect("a run").removed, "b");
    assert_eq!(h.undo().expect("another").removed, "a\n");
}

/// Backspacing runs backwards, and joins into one change the same way.
#[test]
fn a_run_of_backspacing_is_put_back_as_one() {
    let mut h = History::default();
    h.record(deleted(9, "d"));
    h.record(deleted(8, "r"));
    h.record(deleted(7, "o"));
    let back = h.undo().expect("something");
    assert_eq!(back.inserted, "ord", "in the order the file had them");
    assert_eq!(back.at, 7);
}

/// Typing and deleting are different runs even when they are adjacent.
#[test]
fn typing_and_deleting_do_not_join() {
    let mut h = History::default();
    h.record(typed(0, "a"));
    h.record(deleted(0, "a"));
    assert_eq!(h.undo().expect("the delete").inserted, "a");
    assert_eq!(h.undo().expect("the type").removed, "a");
}

/// Moving the caret by hand ends the run: what you type next is a separate
/// thing you did, wherever it lands.
#[test]
fn moving_the_caret_ends_the_run() {
    let mut h = History::default();
    h.record(typed(0, "a"));
    h.breaks();
    h.record(typed(1, "b"));
    assert_eq!(h.undo().expect("second").removed, "b");
    assert_eq!(h.undo().expect("first").removed, "a");
}

#[test]
fn undo_and_redo_are_inverses() {
    let mut h = History::default();
    h.record(typed(4, "hello"));
    let undone = h.undo().expect("undo");
    assert_eq!((undone.at, &undone.removed[..]), (4, "hello"));
    let redone = h.redo().expect("redo");
    assert_eq!((redone.at, &redone.inserted[..]), (4, "hello"));
    assert!(h.redo().is_none(), "nothing further forward");
}

/// You cannot go forward down a road you have just left.
#[test]
fn typing_after_an_undo_drops_what_was_undone() {
    let mut h = History::default();
    h.record(typed(0, "one"));
    h.undo();
    h.record(typed(0, "two"));
    assert!(h.redo().is_none(), "the undone branch is gone");
}

#[test]
fn an_empty_history_undoes_nothing() {
    let mut h = History::default();
    assert!(h.undo().is_none());
    assert!(h.redo().is_none());
}

/// A long session must not grow without bound.
#[test]
fn only_the_last_few_hundred_changes_are_kept() {
    let mut h = History::default();
    for i in 0..(KEEP + 50) {
        h.record(typed(i as u32 * 2, "x"));
        h.breaks();
    }
    assert_eq!(h.done.len(), KEEP);
}

//! A proposed correction is inert until kept: never in a prompt, never on
//! disk; kept, it is tied to the turn it corrected; forgotten, it is gone.
use super::super::Recall;

const FIX: &str = "add a span to the token in crates/lang/src/parser.rs";
const NO: &str = "no, the lexer owns spans, not the parser";
const LATER: &str = "rename Token in crates/lang/src/parser.rs";

/// The turn being corrected, then the user's "no" that names no file.
fn proposed() -> Recall {
    let mut r = Recall::default();
    r.record(FIX, "added Span to Token in crates/lang/src/parser.rs");
    assert!(r.propose_ruled(NO), "the first time it is asked");
    r
}

fn told(r: &Recall) -> String {
    r.context(LATER, &[]).unwrap_or_default()
}

#[test]
fn a_proposed_correction_reaches_no_prompt_and_no_list() {
    let r = proposed();
    assert_eq!(r.proposed(), Some(NO));
    assert!(!told(&r).contains("ruled out"), "{}", told(&r));
    assert!(r.ruled().is_empty());
}

/// The trap: the correcting message is itself recorded before the user
/// answers, so "the turn recorded last" is then the "no", which names no
/// file. Kept, the correction must still reach the parser's next task.
#[test]
fn kept_after_its_own_turn_is_recorded_it_reaches_the_corrected_file() {
    let mut r = proposed();
    r.record(NO, "moved Span to crates/lang/src/lexer.rs instead");
    r.record("keep", "kept");
    assert_eq!(r.keep_proposed().as_deref(), Some(NO));
    assert_eq!(r.proposed(), None);
    let block = told(&r);
    assert!(
        block.contains("- ruled out before (just now): no, the lexer owns spans"),
        "{block}"
    );
}

/// The control for the trap above: the same turns and words, but proposed
/// before the parser turn existed, so no file is tied. It must NOT reach
/// the parser's task — proof that the tie, not the shared word "parser",
/// carried it there.
#[test]
fn kept_with_no_file_tie_the_shared_word_alone_does_not_carry_it() {
    let mut r = Recall::default();
    assert!(r.propose_ruled(NO));
    r.record(FIX, "added Span to Token in crates/lang/src/parser.rs");
    r.record(NO, "moved Span to crates/lang/src/lexer.rs instead");
    assert_eq!(r.keep_proposed().as_deref(), Some(NO));
    assert!(!told(&r).contains("ruled out before"), "{}", told(&r));
}

#[test]
fn forgotten_it_writes_nothing() {
    let mut r = proposed();
    let before = r.nodes();
    assert_eq!(r.drop_proposed().as_deref(), Some(NO));
    assert_eq!(r.nodes(), before);
    assert_eq!(r.keep_proposed(), None, "nothing left to keep");
    assert!(!told(&r).contains("ruled out"));
}

#[test]
fn a_newer_correction_replaces_an_unanswered_one_and_is_asked_once() {
    let mut r = proposed();
    assert!(!r.propose_ruled(NO), "the same words are not asked twice");
    assert!(r.propose_ruled("don't touch Cargo.lock"));
    assert_eq!(r.proposed(), Some("don't touch Cargo.lock"));
    r.keep_proposed();
    assert_eq!(r.ruled().len(), 1, "the passed-over one was not kept");
}

/// Said again after it was kept: the user agreed to these words once, so it
/// is sighted again rather than asked about again.
#[test]
fn a_correction_already_kept_is_not_asked_again() {
    let mut r = proposed();
    r.keep_proposed();
    assert!(!r.propose_ruled(NO));
    assert_eq!(r.proposed(), None);
    assert_eq!(r.ruled().len(), 1);
}

#[test]
fn with_recall_off_nothing_is_proposed() {
    let mut r = Recall::disabled();
    assert!(!r.propose_ruled(NO));
    assert_eq!(r.proposed(), None);
}

/// Longer than a stored correction keeps: found under the same clip, so it
/// is sighted again, not proposed again as if new.
#[test]
fn a_long_correction_already_kept_is_not_asked_again() {
    let long = format!("no, {}", "keep the lexer apart from the parser ".repeat(20));
    let mut r = Recall::default();
    assert!(r.propose_ruled(&long));
    r.keep_proposed();
    assert!(!r.propose_ruled(&long), "asked again");
    assert_eq!(r.ruled().len(), 1);
}

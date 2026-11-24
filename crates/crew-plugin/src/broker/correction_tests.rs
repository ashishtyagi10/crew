//! What becomes a correction, and from where: the router's word, the
//! openings where no router read the message, a compaction's lines.
use std::sync::{Arc, Mutex};

use super::*;
use crate::broker::recall::{lock, Recall};

fn graph() -> SharedRecall {
    let r = Arc::new(Mutex::new(Recall::default()));
    recall::record(
        &r,
        "edit crates/a/src/lib.rs",
        Some("edited crates/a/src/lib.rs"),
    );
    r
}

fn told(r: &SharedRecall) -> String {
    lock(r)
        .context("tidy crates/a/src/lib.rs", &[])
        .unwrap_or_default()
}

#[test]
fn the_openings_catch_a_correction_and_not_a_task() {
    for t in [
        "no, the other file",
        "Don't touch Cargo.lock",
        "that's wrong because the lexer owns it",
        "use the lexer instead",
        "the lexer instead, please",
        "no. the other one",
        "nope",
    ] {
        assert!(backstop(t), "{t}");
    }
    for t in [
        "now run the tests",
        "note this down",
        "add a note to lib.rs",
        "no rush, but tidy the imports",
        "no tests exist for this yet, add some",
        "don't forget to run the tests",
        "use tabs instead of spaces in the new file",
        "add an insteadof rule to the git config",
    ] {
        assert!(!backstop(t), "{t}");
    }
}

/// What the user keeps: the proposal, answered "keep".
fn kept(r: &SharedRecall) -> Option<String> {
    lock(r).keep_proposed()
}

#[test]
fn the_routers_clause_is_proposed_with_the_users_words_and_kept_on_keep() {
    let r = graph();
    let asked = note(
        &r,
        Some(Some("editing lib.rs by hand")),
        "regenerate it, don't edit",
    )
    .expect("a correction is proposed in one line");
    assert!(asked.contains("editing lib.rs by hand"), "{asked}");
    assert!(asked.contains("\u{201c}keep\u{201d}") && asked.contains("\u{201c}forget\u{201d}"));
    assert!(!told(&r).contains("ruled out"), "proposed is not kept");
    kept(&r).expect("it was waiting");
    let block = told(&r);
    assert!(block.contains("ruled out before"), "{block}");
    assert!(block.contains("editing lib.rs by hand"), "{block}");
    assert!(block.contains("regenerate it"), "{block}");
}

#[test]
fn a_router_that_read_the_message_and_said_nothing_is_trusted() {
    let r = graph();
    assert_eq!(note(&r, Some(None), "no, the other file"), None);
    assert_eq!(kept(&r), None);
}

#[test]
fn with_no_router_the_openings_decide() {
    let r = graph();
    assert_eq!(note(&r, None, "add a docstring"), None);
    assert!(note(&r, None, "no, the other file").is_some());
    kept(&r);
    assert!(told(&r).contains("ruled out before"), "{}", told(&r));
}

#[test]
fn a_summarys_ruled_out_lines_are_read_and_none_is_not_one() {
    let s =
        "done: edited lib.rs\n- ruled out: serde 2.0 bump — E0277.\nRuled out: none\nopen: tests";
    assert_eq!(ruled_lines(s), vec!["serde 2.0 bump — E0277".to_string()]);
}

#[test]
fn a_compaction_writes_what_it_ruled_out_into_the_graph() {
    let r = graph();
    let call: Summarize =
        Arc::new(|_: &str| Ok("done: x\nruled out: patching crates/a/src/lib.rs by hand".into()));
    let wrapped = remembering(call, Arc::clone(&r));
    assert!(wrapped("req").unwrap().contains("ruled out"));
    let block = told(&r);
    assert!(
        block.contains("ruled out before (just now): patching crates/a/src/lib.rs"),
        "{block}"
    );
}

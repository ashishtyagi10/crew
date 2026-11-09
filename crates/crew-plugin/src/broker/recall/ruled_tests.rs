//! A correction is a node the walk reaches: in front of the next task on the
//! same files, labelled; never in front of unrelated work or its own message.
use super::super::Recall;

const FIX: &str = "add a span to the token in crates/lang/src/parser.rs";

/// The turn being corrected, then the user's "no" that names no file.
fn corrected() -> Recall {
    let mut r = Recall::default();
    r.record(FIX, "added Span to Token in crates/lang/src/parser.rs");
    r.record_ruled("no, the lexer owns spans, not the parser", true);
    r
}

#[test]
fn a_later_task_on_the_corrected_file_is_told_what_was_ruled_out() {
    let r = corrected();
    let block = r
        .context("rename Token in crates/lang/src/parser.rs", &[])
        .expect("nothing recalled");
    assert!(
        block.contains("- ruled out before (just now): no, the lexer owns spans"),
        "{block}"
    );
}

#[test]
fn an_unrelated_task_is_not_told() {
    let r = corrected();
    assert!(r.context("what is the weather in berlin", &[]).is_none());
    let other = r.context("bump the version in docs/install.md", &[]);
    assert!(
        !other.unwrap_or_default().contains("ruled out"),
        "an unrelated file recalled the correction"
    );
}

/// Four turns that share a rare word with the request outscore the
/// correction (joined by the file alone); it still leads, and is not the one
/// the four-turn cut drops.
#[test]
fn a_correction_comes_before_the_turns_it_competes_with() {
    let mut r = corrected();
    for n in 0..4 {
        r.record(
            &format!("spanless tokens pass {n} in crates/lang/src/parser.rs"),
            &format!("spanless pass {n} done"),
        );
    }
    let block = r
        .context("spanless tokens in crates/lang/src/parser.rs", &[])
        .unwrap();
    let ruled = block.find("ruled out before").expect(&block);
    let turn = block.find("you asked").unwrap();
    assert!(ruled < turn, "{block}");
}

#[test]
fn a_correction_is_not_recalled_in_front_of_the_message_that_said_it() {
    let r = corrected();
    let said = "no, the lexer owns spans, not the parser";
    let block = r.context(said, &[]).unwrap_or_default();
    assert!(!block.contains("ruled out"), "{block}");
}

#[test]
fn the_same_words_twice_are_one_node() {
    let mut r = corrected();
    let before = r.nodes();
    r.record_ruled("no, the lexer owns spans, not the parser", true);
    assert_eq!(r.nodes(), before);
}

#[test]
fn a_correction_is_never_what_a_pane_opening_quotes() {
    let r = corrected();
    let (text, _) = r.latest("").unwrap();
    assert!(text.starts_with("you asked: add a span"), "{text}");
}

#[test]
fn an_untied_line_reaches_only_what_it_names() {
    let mut r = Recall::default();
    r.record(FIX, "added Span to Token in crates/lang/src/parser.rs");
    r.record_ruled("serde 2.0 bump: E0277 in crates/lang/src/de.rs", false);
    let parser = r.context("crates/lang/src/parser.rs", &[]).unwrap();
    assert!(!parser.contains("ruled out"), "{parser}");
    let de = r.context("fix crates/lang/src/de.rs", &[]).unwrap();
    assert!(de.contains("ruled out before"), "{de}");
}

use super::*;

use crate::broker::recall;

fn session() -> Session {
    let session = Session::default();
    {
        let mut r = recall::lock(&session.recall);
        r.record("edit crates/a/src/lib.rs", "edited crates/a/src/lib.rs");
        r.record_ruled("patching crates/a/src/lib.rs by hand", true);
        r.record_ruled("the serde 2.0 bump", false);
    }
    session
}

#[test]
fn the_questions_are_recognised_whole_and_a_mention_is_a_task() {
    assert_eq!(asks("What did I rule out?"), Some(Ask::List));
    assert_eq!(asks("show corrections"), Some(Ask::List));
    assert_eq!(asks("forget correction 2"), Some(Ask::Forget(2)));
    assert_eq!(asks("Forget ruled out #1."), Some(Ask::Forget(1)));
    for task in [
        "what did I rule out in the parser design doc",
        "forget correction",
        "forget correction two",
        "forget the correction about serde",
        "list corrections the reviewer asked for",
    ] {
        assert_eq!(asks(task), None, "{task:?}");
    }
}

#[test]
fn the_list_numbers_what_is_ruled_out_newest_first() {
    let text = answer(&session(), Ask::List);
    let one = text.find("1. (just now) the serde 2.0 bump");
    let two = text.find("2. (just now) patching crates/a/src/lib.rs by hand");
    assert!(one.is_some() && one < two, "{text}");
    assert!(text.contains("forget correction 1"), "{text}");
}

#[test]
fn forgetting_names_what_went_and_the_list_is_one_shorter() {
    let s = session();
    assert_eq!(
        answer(&s, Ask::Forget(1)),
        "forgot correction 1: the serde 2.0 bump"
    );
    let text = answer(&s, Ask::List);
    assert!(!text.contains("serde"), "{text}");
    assert!(text.contains("1. (just now) patching"), "{text}");
}

#[test]
fn a_number_that_is_not_on_the_list_says_how_many_are() {
    let s = session();
    assert_eq!(
        answer(&s, Ask::Forget(9)),
        "there is no correction 9 \u{2014} 2 kept"
    );
    assert_eq!(
        answer(&Session::default(), Ask::List),
        "nothing is ruled out in this project"
    );
}

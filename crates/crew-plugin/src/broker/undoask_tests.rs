use super::*;

#[test]
fn the_plain_ways_of_saying_put_it_back_are_all_undo() {
    for phrase in [
        "undo",
        "undo that",
        "undo it.",
        "Undo That!",
        "revert that",
        "put it back",
        "roll that back",
        "take that back",
    ] {
        assert_eq!(
            asks_undo(phrase),
            Some(0),
            "{phrase:?} did not read as undo"
        );
    }
}

#[test]
fn an_ordinal_goes_further_back_and_is_one_based_like_the_list() {
    assert_eq!(asks_undo("undo #3"), Some(2));
    assert_eq!(asks_undo("undo that #1"), Some(0));
    assert_eq!(
        asks_undo("undo #0"),
        Some(0),
        "there is no zeroth checkpoint"
    );
}

#[test]
fn a_task_that_merely_contains_the_word_undo_is_a_task() {
    // The whole risk of this gate: "undo the nav change" asks an AGENT to
    // make a change. Reverting the tree instead would destroy work.
    for phrase in [
        "undo the nav change",
        "can you undo the last commit in git",
        "revert the migration in db/schema.sql",
        "put it back the way it was in the header",
        "undo everything and start over with a new design",
    ] {
        assert_eq!(asks_undo(phrase), None, "{phrase:?} was taken as an undo");
    }
}

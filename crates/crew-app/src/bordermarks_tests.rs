use super::*;

#[test]
fn the_switch_starts_on_because_the_marks_are_the_default() {
    // Set explicitly rather than assumed: another test may have moved it.
    set(true);
    assert!(on());
    set(false);
    assert!(!on());
    set(true);
}

#[test]
fn every_spelling_of_an_answer_parses_and_nothing_else_does() {
    for yes in ["on", "ON", " yes ", "true"] {
        assert_eq!(parse(yes), Some(true), "{yes}");
    }
    for no in ["off", "No", "false"] {
        assert_eq!(parse(no), Some(false), "{no}");
    }
    for neither in ["", "maybe", "1", "auto"] {
        assert_eq!(parse(neither), None, "{neither}");
    }
}

use super::*;

#[test]
fn met_ends_it_even_when_the_gap_would_repeat() {
    assert_eq!(step(Some("a"), (true, "a")), Next::Met);
}

#[test]
fn the_first_miss_always_goes_back() {
    let next = step(None, (false, " the tests were never run "));
    assert_eq!(next, Next::Revise("the tests were never run".into()));
}

/// Case and punctuation do not make a gap new; different words do.
#[test]
fn a_repeat_is_the_same_words_not_the_same_bytes() {
    let next = step(Some("`c.rs` still uses X."), (false, "C.rs still uses x"));
    assert_eq!(next, Next::Stalled("no progress: C.rs still uses x".into()));
    let next = step(Some("c.rs still uses x"), (false, "d.rs still uses x"));
    assert_eq!(next, Next::Revise("d.rs still uses x".into()));
}

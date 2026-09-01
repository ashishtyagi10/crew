//! The network is not exercised here — that is what an integration test with a live server is
//! for — but the clipping and the failure text are, because both reach the model.
use super::*;

#[test]
fn a_short_response_is_returned_as_it_is() {
    assert_eq!(clip("hello"), "hello");
}

#[test]
fn a_huge_response_is_cut_on_a_character_boundary_and_says_so() {
    // A JSON API can answer with a megabyte, and a tool result is prompt text.
    let text = "\u{00e9}".repeat(CAP);
    let out = clip(&text);
    assert!(out.len() < text.len());
    assert!(out.contains("truncated"), "it admits the cut");
    assert!(
        out.chars().count() > 1,
        "and it is still valid text: {}",
        out.len()
    );
}

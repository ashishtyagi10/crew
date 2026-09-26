//! A dash stays with the word before it, at every width.
use super::*;

const ASK: &str = "Try \u{201c}make the tests pass\u{201d} \u{2014} a swarm that verifies \
    its own result \u{2014} or \u{201c}draft a plan first\u{201d} \u{2014} nothing runs \
    until you approve.";

/// At the composer's width the break fell after `result`, and the next row
/// opened on the dash that belonged to it.
#[test]
fn no_row_starts_with_a_dash() {
    for cols in 16..=140u16 {
        for row in wrap_to(ASK, cols) {
            assert!(!row.starts_with('\u{2014}'), "{cols}: {row:?}");
            assert!(
                row.chars().count() <= wrap_width(cols) || !row.contains(' '),
                "{cols}: {row:?}"
            );
        }
    }
}

/// The words survive the gluing, in order.
#[test]
fn every_word_is_kept() {
    for cols in [24u16, 60, 72, 120] {
        let joined = wrap_to(ASK, cols).join(" ");
        assert_eq!(joined, ASK.split_whitespace().collect::<Vec<_>>().join(" "));
    }
}

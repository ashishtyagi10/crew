//! A key longer than the entry card's field says so.
use super::KeyEntry;
use crate::chatkeys::ChatInput;

/// The mask fills the field and ends in `…` when the key is longer than it,
/// rather than stopping at the edge as if the key ended there; a key that
/// fits is all dots.
#[test]
fn a_key_longer_than_the_field_ends_in_an_ellipsis() {
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    for c in "x".repeat(40).chars() {
        e.key(&ChatInput::Char(c));
    }
    let row: String = e
        .card(32)
        .cells
        .iter()
        .filter(|c| c.row == 1)
        .map(|c| c.c)
        .collect();
    assert!(row.contains('\u{2026}'), "{row:?}");
    let mut short = KeyEntry::new("ANTHROPIC_API_KEY".into());
    for c in "xyz".chars() {
        short.key(&ChatInput::Char(c));
    }
    let row: String = short
        .card(60)
        .cells
        .iter()
        .filter(|c| c.row == 1)
        .map(|c| c.c)
        .collect();
    assert!(
        !row.contains('\u{2026}') && row.matches('•').count() == 3,
        "{row:?}"
    );
}

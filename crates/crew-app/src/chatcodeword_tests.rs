//! The code line's word: one a turn, decoding out of glyphs.
use super::*;

#[test]
fn letters_lock_in_left_to_right_behind_a_bright_head() {
    let ms = |b: u64| b;
    let at = SCRAMBLE_MS + 2 * LOCK_MS + 10;
    let got: Vec<Slot> = (0..5).map(|i| slot(i, 5, at, 7, ms)).collect();
    use Slot::*;
    assert_eq!(got, [Letter, Letter, Head, Glyph, Glyph]);
    assert!(
        (0..5).all(|i| slot(i, 5, 10, 7, ms) == Glyph),
        "it opens as glyphs"
    );
    let held = SCRAMBLE_MS + 5 * LOCK_MS + 100;
    assert!(
        (0..5).all(|i| slot(i, 5, held, 7, ms) == Letter),
        "then holds"
    );
    let gone = SCRAMBLE_MS + 5 * LOCK_MS + HOLD_MS + DISSOLVE_MS;
    assert!(
        (0..5).all(|i| slot(i, 5, gone, 7, ms) == Glyph),
        "and dissolves"
    );
}

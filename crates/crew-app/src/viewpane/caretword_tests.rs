//! Word hops, on the very rows the pane draws.
use crate::viewpane::caret::tests::{char_at, lines, DOC};
use crate::viewpane::caret::{first, offset_at, step, Step};

#[test]
fn a_word_right_lands_after_the_word_and_a_word_left_on_its_first_letter() {
    let ls = lines(DOC, 40);
    let mut c = step(&ls, first(&ls).expect("start"), Step::Bottom);
    c = step(&ls, c, Step::Home);
    assert_eq!(char_at(&ls, c), Some('S'), "`Some prose here.`");
    c = step(&ls, c, Step::WordRight);
    assert_eq!(char_at(&ls, c), Some(' '), "on the blank after `Some`");
    c = step(&ls, c, Step::WordRight);
    assert_eq!(char_at(&ls, c), Some(' '), "after `prose`");
    c = step(&ls, c, Step::WordRight);
    assert_eq!(char_at(&ls, c), None, "after `here.` — the row's end");
    assert_eq!(offset_at(&ls, c), Some(DOC.len() as u32 - 1));
    c = step(&ls, c, Step::WordLeft);
    assert_eq!(char_at(&ls, c), Some('h'));
    c = step(&ls, c, Step::WordLeft);
    assert_eq!(char_at(&ls, c), Some('p'));
    c = step(&ls, c, Step::WordLeft);
    assert_eq!(char_at(&ls, c), Some('S'));
}

/// From a row's end the hop is the ordinary step onto the next row, so a
/// document can be walked by words alone.
#[test]
fn at_a_rows_end_the_hop_steps_onto_the_next_row() {
    let ls = lines(DOC, 40);
    let start = first(&ls).expect("start");
    let end = step(&ls, start, Step::WordRight);
    assert_eq!(char_at(&ls, end), None, "after `Title`");
    assert_eq!(offset_at(&ls, end), Some(7), "`# Title` is seven bytes");
    let over = step(&ls, end, Step::WordRight);
    assert_eq!(over, step(&ls, end, Step::Right), "onto the next row");
    assert_ne!(over.row, end.row);
    assert_eq!(
        step(&ls, start, Step::WordLeft),
        start,
        "the first place holds"
    );
    let back = step(&ls, over, Step::WordLeft);
    assert_eq!(
        back,
        step(&ls, over, Step::Left),
        "and back over the row break"
    );
}

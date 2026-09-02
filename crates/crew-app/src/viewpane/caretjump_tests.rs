//! The long steps: a page at a time, and the document's two ends.
use super::tests::{char_at, lines, DOC};
use super::*;

/// A page is the window's height in places, and the ends hold.
#[test]
fn a_page_moves_that_many_rows_and_the_ends_hold() {
    let ls = lines(DOC, 40);
    let c = first(&ls).expect("start");
    let down = |c, rows| step(&ls, c, Step::Page { down: true, rows });
    let one = down(c, 1);
    assert_eq!(char_at(&ls, one), Some('o'), "one row: the first bullet");
    let two = down(c, 2);
    assert_eq!(char_at(&ls, two), Some('t'), "two rows: the second bullet");
    let far = down(c, 50);
    assert_eq!(char_at(&ls, far), Some('S'), "past the end: the last row");
    assert_eq!(down(far, 3), far, "the last row holds");
    let back = step(
        &ls,
        far,
        Step::Page {
            down: false,
            rows: 50,
        },
    );
    assert_eq!(back, c, "and a page up from there is the start");
}

#[test]
fn top_and_bottom_are_the_documents_ends() {
    let ls = lines(DOC, 40);
    let mut c = first(&ls).expect("start");
    for _ in 0..3 {
        c = step(&ls, c, Step::Right);
    }
    let end = step(&ls, c, Step::Bottom);
    assert_eq!(char_at(&ls, end), None, "after the last character");
    assert_eq!(
        offset_at(&ls, end),
        Some(DOC.len() as u32 - 1),
        "the final newline's byte"
    );
    let top = step(&ls, end, Step::Top);
    assert_eq!(char_at(&ls, top), Some('T'));
    assert_eq!(step(&ls, top, Step::Top), top, "the first place holds");
}

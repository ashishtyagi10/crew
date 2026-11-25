//! The code and plain rungs carry each character's byte, and a blank line one place to stand.
use super::*;
use crate::viewpane::caret::stops;

fn bytes_of(rows: &[CardLine]) -> Vec<Vec<(u16, u32)>> {
    rows.iter().map(stops).collect()
}

/// A numbered rung: every body character is its byte, the gutter is nothing, a blank line is
/// one stop on its own newline, and each line ends on a stop past its last character.
#[test]
fn the_numbered_rung_carries_bytes() {
    let ink = (200, 200, 200);
    let rows = crate::viewpane::linepaint::numbered("ab\n\ncd", 40, "", ink, ink, &[]);
    let g = crate::viewpane::linepaint::GUTTER_W as u16;
    assert_eq!(
        bytes_of(&rows),
        vec![
            vec![(g, 0), (g + 1, 1), (g + 2, 2)],
            vec![(g, 3)],
            vec![(g, 4), (g + 1, 5), (g + 2, 6)],
        ]
    );
}

/// A wrapped line's second row goes on from the byte the first one stopped at.
#[test]
fn a_wrapped_row_goes_on_from_its_byte() {
    let ink = (200, 200, 200);
    let w = crate::viewpane::linepaint::GUTTER_W + 4;
    let rows = crate::viewpane::linepaint::numbered("abcdefgh", w, "", ink, ink, &[]);
    let firsts: Vec<u32> = rows.iter().map(|r| stops(r)[0].1).collect();
    assert_eq!(firsts, vec![0, 4]);
}

/// The plain rung, the same: bytes on its characters, an anchor on its blank line.
#[test]
fn the_plain_rung_carries_bytes() {
    let ink = (200, 200, 200);
    let rows = crate::viewpane::plainrung::unnumbered("hi\n\nyo", 40, ink, ink, &[]);
    assert_eq!(
        bytes_of(&rows),
        vec![
            vec![(0, 0), (1, 1), (2, 2)],
            vec![(0, 3)],
            vec![(0, 4), (1, 5), (2, 6)]
        ]
    );
}

/// A space the file holds is a character, not an anchor: it still has a stop after it.
#[test]
fn a_typed_space_is_not_an_anchor() {
    let mut c = plain(ANCHOR_C, (0, 0, 0), false);
    c.src = Some(0);
    assert!(!is_anchor(&c));
    assert_eq!(stops(&vec![c]), vec![(0, 0), (1, 1)]);
}

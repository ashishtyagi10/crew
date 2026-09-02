//! Where a caret can be, and where it must refuse to be.
use super::super::caretfind::find;
use super::*;
use crate::md::render;

/// Render `text` at `cols` the way the viewer's markdown rung does, so these
/// are the very lines the pane draws.
pub(super) fn lines(text: &str, cols: usize) -> Vec<CardLine> {
    let fg = crew_theme::theme().ink;
    crate::chatmd::map_lines(render(text, cols), cols, fg)
}

/// The character the caret is on — `None` at a row's end stop, where the
/// caret stands after the last character and there is no cell.
pub(super) fn char_at(ls: &[CardLine], c: Caret) -> Option<char> {
    let mut col = 0u16;
    for cell in ls.get(c.row)? {
        let w = crate::chatwidth::char_w(cell.c) as u16;
        if w == 0 {
            continue;
        }
        if col == c.col {
            return Some(cell.c);
        }
        col += w;
    }
    None
}

pub(super) const DOC: &str = "\
# Title

- one
- two

Some prose here.
";

#[test]
fn the_caret_starts_on_the_first_character_of_the_document() {
    let ls = lines(DOC, 40);
    let c = first(&ls).expect("somewhere to start");
    assert_eq!(char_at(&ls, c), Some('T'), "the heading's first letter");
    assert_eq!(offset_at(&ls, c), Some(2), "`# ` is two bytes of source");
}

/// The rule the whole design rests on: a character the renderer invented is
/// not a place the caret can go.
#[test]
fn the_caret_steps_over_what_the_renderer_added() {
    let ls = lines("- one\n- two\n", 40);
    let mut c = first(&ls).expect("start");
    assert_eq!(char_at(&ls, c), Some('o'), "not the bullet");
    // Six characters of document and two end stops: the two items' text, the
    // place after each, and nothing else. The bullets, the spaces after them
    // and the line breaks are all furniture.
    let mut seen = String::new();
    for _ in 0..8 {
        seen.push(char_at(&ls, c).unwrap_or('\u{2423}'));
        c = step(&ls, c, Step::Right);
    }
    assert_eq!(seen, "one\u{2423}two\u{2423}", "walked: {seen:?}");
    assert!(!seen.contains('\u{2022}'), "the caret sat on a bullet");
}

/// …and every place it CAN go is a byte of the file, holding that character.
#[test]
fn every_position_the_caret_walks_is_its_own_byte() {
    let ls = lines(DOC, 40);
    let mut c = first(&ls).expect("start");
    for _ in 0..60 {
        if let (Some(ch), Some(at)) = (char_at(&ls, c), offset_at(&ls, c)) {
            assert_eq!(
                DOC[at as usize..].chars().next(),
                Some(ch),
                "caret on {ch:?} claimed byte {at}"
            );
        }
        c = step(&ls, c, Step::Right);
    }
}

#[test]
fn left_and_right_are_inverses_in_the_middle_of_a_document() {
    let ls = lines(DOC, 40);
    let start = first(&ls).expect("start");
    let mut c = start;
    for _ in 0..12 {
        c = step(&ls, c, Step::Right);
    }
    let there = c;
    for _ in 0..12 {
        c = step(&ls, c, Step::Left);
    }
    assert_eq!(c, start, "walked out to {there:?} and back to {c:?}");
}

/// A cursor that wraps from the last character to the first is a cursor
/// nobody can hold on to.
#[test]
fn the_ends_of_the_document_hold() {
    let ls = lines(DOC, 40);
    let mut c = first(&ls).expect("start");
    assert_eq!(step(&ls, c, Step::Left), c, "the first place holds");
    for _ in 0..200 {
        c = step(&ls, c, Step::Right);
    }
    assert_eq!(step(&ls, c, Step::Right), c, "the last place holds");
    assert_eq!(
        char_at(&ls, c),
        None,
        "…which is the place AFTER the last character, where a document is \
         appended to"
    );
    assert_eq!(
        offset_at(&ls, c),
        Some(DOC.len() as u32 - 1),
        "one past the final `.`, before the trailing newline"
    );
}

/// Vertical movement aims for the column it started from, not the one the
/// short line it passed through happened to end at.
#[test]
fn down_through_a_short_line_comes_back_to_the_column_it_left() {
    // Separate PARAGRAPHS, not lines: CommonMark joins a soft break with a
    // space, so three consecutive lines are one paragraph and would wrap into
    // whatever shape the width gives them.
    let ls = lines(
        "a long first line of prose\n\nx\n\nanother long line of prose\n",
        40,
    );
    let mut c = first(&ls).expect("start");
    for _ in 0..10 {
        c = step(&ls, c, Step::Right);
    }
    let col = c.col;
    let down = step(&ls, c, Step::Down);
    let down2 = step(&ls, down, Step::Down);
    assert!(down.col < col, "the short line cannot reach column {col}");
    assert_eq!(down2.col, col, "…but the long one below it can");
}

#[test]
fn home_and_end_land_on_the_ends_of_the_row() {
    let ls = lines("Some prose here.\n", 40);
    let c = first(&ls).expect("start");
    let end = step(&ls, c, Step::End);
    assert_eq!(char_at(&ls, end), None, "End is past the last character");
    assert_eq!(char_at(&ls, step(&ls, end, Step::Left)), Some('.'));
    assert_eq!(step(&ls, end, Step::Home), c);
}

/// After a re-layout the caret has to be found again from its byte — and it
/// has to be a search, not a walk: a hundred-thousand-row document must not
/// be scanned to answer it.
#[test]
fn a_caret_is_found_again_from_its_offset_after_a_relayout() {
    let wide = lines(DOC, 60);
    let narrow = lines(DOC, 12);
    let mut c = first(&wide).expect("start");
    for _ in 0..20 {
        c = step(&wide, c, Step::Right);
    }
    let at = offset_at(&wide, c).expect("an offset");
    let moved = find(&narrow, at).expect("found again");
    assert_eq!(
        offset_at(&narrow, moved),
        Some(at),
        "the same byte, at a different width"
    );
}

/// An offset the new layout no longer holds (the character was deleted) must
/// land on the next one that exists rather than nowhere.
#[test]
fn an_offset_that_is_gone_lands_on_the_next_one() {
    let ls = lines("abc\n\ndef\n", 40);
    let found = find(&ls, 3).expect("something after the deleted byte");
    assert!(offset_at(&ls, found).is_some_and(|s| s >= 3));
}

#[test]
fn a_document_with_nowhere_to_stand_has_no_caret() {
    assert_eq!(first(&lines("---\n", 40)), None, "a rule is all furniture");
    assert_eq!(first(&[]), None);
}

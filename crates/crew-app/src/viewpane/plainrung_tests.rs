use super::{unnumbered, MARK};

/// A row as the source wrote it: the continuation mark is the viewer's, not
/// the text's.
fn unmarked(row: &str) -> &str {
    row.trim_start()
        .strip_prefix(MARK)
        .map(str::trim_start)
        .unwrap_or_else(|| row.trim_start())
}

fn text_of(rows: &[crate::chatbody::CardLine]) -> Vec<String> {
    rows.iter()
        .map(|r| r.iter().map(|c| c.c).collect::<String>())
        .collect()
}

#[test]
fn prose_breaks_between_words_and_a_detail_keeps_its_indent() {
    let ink = (200, 200, 200);
    let text = "a row that is longer than the pane is wide\n     detail under it, also long enough to wrap";
    let rows = text_of(&unnumbered(text, 20, ink, ink, &[]));
    assert!(rows.len() >= 5, "{rows:?}");
    for row in &rows {
        assert!(row.chars().count() <= 20, "{row:?}");
        // A whole-word piece of the text: what follows it in the source is a
        // space or the end, never the rest of a word.
        let piece = unmarked(row);
        let at = text
            .find(piece)
            .unwrap_or_else(|| panic!("{row:?} is not in the text"));
        let after = text[at + piece.len()..].chars().next();
        assert!(
            matches!(after, None | Some(' ') | Some('\n')),
            "{row:?} ends mid-word"
        );
    }
    assert_eq!(rows[0].trim_end(), "a row that is");
    let detail = rows
        .iter()
        .position(|r| r.starts_with("     detail"))
        .expect("the detail line");
    assert!(
        rows[detail + 1].starts_with("     \u{21aa} "),
        "the continuation keeps the indent and says it is one: {:?}",
        rows[detail + 1]
    );
}

#[test]
fn a_blank_line_is_one_empty_row_and_nothing_is_numbered() {
    let ink = (1, 2, 3);
    let rows = text_of(&unnumbered("one\n\nthree", 40, ink, ink, &[]));
    assert_eq!(rows, vec!["one", "", "three"]);
}

/// Every character of an indented line survives the wrap. The continuation
/// used to be wrapped at the full width and then cut back to it AFTER its
/// indent was prepended — the last `indent` characters of each such row were
/// dropped, and nothing marked the cut.
#[test]
fn an_indented_continuation_loses_nothing() {
    let ink = (200, 200, 200);
    let text = "        eight in: alpha bravo charlie delta echo foxtrot golf hotel india juliet";
    let rows = text_of(&unnumbered(text, 24, ink, ink, &[]));
    let mut seen = String::new();
    for row in &rows {
        assert!(row.chars().count() <= 24, "{row:?}");
        seen.push_str(unmarked(row).trim_end());
        seen.push(' ');
    }
    for word in text.split_whitespace() {
        assert!(seen.contains(word), "{word} was dropped: {rows:?}");
    }
    // …and a long unbreakable word is hard-cut into pieces that all arrive.
    let rows = text_of(&unnumbered(
        "    abcdefghijklmnopqrstuvwxyz0123456789",
        12,
        ink,
        ink,
        &[],
    ));
    let joined: String = rows.iter().map(|r| unmarked(r).trim_end()).collect();
    assert_eq!(joined, "abcdefghijklmnopqrstuvwxyz0123456789", "{rows:?}");
}

/// The defect this rung had: a listing row that wrapped read as the next row.
/// `/watching` at a tile width put `calendar` in column zero under `w1 … brief
/// me on the`, which is what the next standing intent looks like.
#[test]
fn a_wrapped_listing_row_cannot_be_read_as_the_next_one() {
    let ink = (200, 200, 200);
    let text = "w1   in 2h    daily     brief me on the calendar\nw2   in 5d    once      chase the invoice";
    let rows = text_of(&unnumbered(text, 40, ink, ink, &[]));
    assert!(rows.len() > 2, "the first row wrapped: {rows:?}");
    for row in &rows {
        let starts_a_row = row.starts_with('w');
        assert!(
            starts_a_row || row.trim_start().starts_with(MARK),
            "{row:?} is neither a row nor marked as a continuation"
        );
    }
}

/// The mark costs the row two columns, which come out of the wrap rather than
/// off the edge of the card.
#[test]
fn no_row_is_wider_than_the_pane() {
    let ink = (9, 9, 9);
    let text = "     an indented detail line with plenty of words in it to wrap several times over";
    for cols in 4..40usize {
        for row in text_of(&unnumbered(text, cols, ink, ink, &[])) {
            assert!(row.chars().count() <= cols, "{cols}: {row:?}");
        }
    }
}

/// A pane too narrow for the mark keeps the words whole instead: the mark is
/// paid for out of the row, and a wrap you can see is not worth a word you
/// cannot read. `alpha bravo charlie` at six columns is three rows either
/// way — with the mark it would be five, two of them half a word.
#[test]
fn a_pane_with_no_room_for_the_mark_keeps_the_words() {
    let ink = (9, 9, 9);
    let rows = text_of(&unnumbered("alpha bravo eight", 6, ink, ink, &[]));
    assert_eq!(rows, vec!["alpha", "bravo", "eight"], "{rows:?}");
}

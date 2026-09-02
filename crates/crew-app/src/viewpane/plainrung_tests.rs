use super::unnumbered;

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
        let piece = row.trim_start();
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
        rows[detail + 1].starts_with("     ") && !rows[detail + 1].starts_with("      "),
        "the continuation keeps the indent: {:?}",
        rows[detail + 1]
    );
}

#[test]
fn a_blank_line_is_one_empty_row_and_nothing_is_numbered() {
    let ink = (1, 2, 3);
    let rows = text_of(&unnumbered("one\n\nthree", 40, ink, ink, &[]));
    assert_eq!(rows, vec!["one", "", "three"]);
}

use super::*;

fn cell(col: u16, row: u16, c: char) -> CellView {
    CellView {
        col,
        row,
        c,
        fg: (0, 0, 0),
        bg: (1, 1, 1),
        bold: false,
        italic: false,
        ..Default::default()
    }
}

/// "hello" on row 0, "world" on row 1.
fn grid() -> Vec<CellView> {
    let mut v = Vec::new();
    for (i, c) in "hello".chars().enumerate() {
        v.push(cell(i as u16, 0, c));
    }
    for (i, c) in "world".chars().enumerate() {
        v.push(cell(i as u16, 1, c));
    }
    v
}

#[test]
fn single_row_partial_selection() {
    let sel = CellSel {
        pane: 0,
        anchor: (1, 0),
        cursor: (3, 0),
    };
    assert_eq!(selection_text(&grid(), &sel), "ell");
}

#[test]
fn selection_is_direction_agnostic() {
    let fwd = CellSel {
        pane: 0,
        anchor: (1, 0),
        cursor: (3, 0),
    };
    let rev = CellSel {
        pane: 0,
        anchor: (3, 0),
        cursor: (1, 0),
    };
    assert_eq!(selection_text(&grid(), &fwd), selection_text(&grid(), &rev));
}

#[test]
fn multi_row_selection_joins_with_newline() {
    // From row 0 col 2 through row 1 col 2: "llo" + "wor".
    let sel = CellSel {
        pane: 0,
        anchor: (2, 0),
        cursor: (2, 1),
    };
    assert_eq!(selection_text(&grid(), &sel), "llo\nwor");
}

#[test]
fn gaps_become_spaces_and_trailing_trimmed() {
    // "a" at col 0, "b" at col 3 on row 0; select the whole row.
    let cells = vec![cell(0, 0, 'a'), cell(3, 0, 'b')];
    let sel = CellSel {
        pane: 0,
        anchor: (0, 0),
        cursor: (9, 0),
    };
    assert_eq!(selection_text(&cells, &sel), "a  b");
}

#[test]
fn highlight_only_touches_selected_cells() {
    let mut cells = grid();
    let sel = CellSel {
        pane: 0,
        anchor: (0, 0),
        cursor: (1, 0),
    };
    highlight(&mut cells, &sel, (9, 9, 9));
    // Row 0 cols 0,1 washed; everything else keeps its original bg.
    for c in &cells {
        let washed = c.row == 0 && c.col <= 1;
        assert_eq!(c.bg == (9, 9, 9), washed, "cell {},{}", c.col, c.row);
    }
}

#[test]
fn empty_when_selection_misses_all_glyphs() {
    let sel = CellSel {
        pane: 0,
        anchor: (20, 5),
        cursor: (25, 5),
    };
    assert_eq!(selection_text(&grid(), &sel), "");
}

/// A row of text with two words and a path, laid out at known columns.
fn line(text: &str, row: u16) -> Vec<CellView> {
    text.chars()
        .enumerate()
        .filter(|(_, c)| *c != ' ')
        .map(|(i, c)| cell(i as u16, row, c))
        .collect()
}

#[test]
fn a_word_span_stops_at_the_spaces_around_it() {
    let v = line("hello world", 0);
    assert_eq!(word_span(&v, 0, 0), Some((0, 4)), "the first word");
    assert_eq!(word_span(&v, 3, 0), Some((0, 4)), "clicked mid-word");
    assert_eq!(word_span(&v, 6, 0), Some((6, 10)), "the second word");
    assert_eq!(word_span(&v, 10, 0), Some((6, 10)), "its last glyph");
}

/// The point of double-clicking a path is getting the path, not one segment
/// of it — `/` is not a separator, matching what the terminal panes do.
#[test]
fn a_path_is_one_word() {
    let v = line("cd /usr/local/bin", 0);
    assert_eq!(word_span(&v, 8, 0), Some((3, 16)));
}

/// A separator under the cursor selects nothing, rather than reaching for
/// the nearest word — clicking empty space should not grab text.
#[test]
fn a_click_on_a_gap_or_a_separator_selects_nothing() {
    let v = line("hello world", 0);
    assert_eq!(word_span(&v, 5, 0), None, "the space between them");
    assert_eq!(word_span(&v, 40, 0), None, "past the end of the row");
    let punct = line("a:b", 0);
    assert_eq!(word_span(&punct, 1, 0), None, "on the colon itself");
    assert_eq!(word_span(&punct, 0, 0), Some((0, 0)), "and it splits them");
    assert_eq!(word_span(&punct, 2, 0), Some((2, 2)));
}

#[test]
fn a_line_span_reaches_from_the_first_glyph_to_the_last() {
    let v = line("hello world", 0);
    assert_eq!(line_span(&v, 0), Some((0, 10)));
    assert_eq!(line_span(&v, 1), None, "a row with nothing drawn on it");
}

/// The two gestures must not collapse into each other: a line is at least a
/// word, and on a multi-word row it is strictly more.
#[test]
fn the_line_span_is_wider_than_the_word_under_the_same_cell() {
    let v = line("hello world", 0);
    let (wl, wh) = word_span(&v, 1, 0).unwrap();
    let (ll, lh) = line_span(&v, 0).unwrap();
    assert!(ll <= wl && lh > wh, "{ll}..{lh} should contain {wl}..{wh}");
}

//! A markdown table's colours: the header in the accent, the grid — the
//! rule under the header and the column separators — muted and one colour.
//! Split from `chatspan_tests` (at the line cap).
use crate::chatbody::{CardLine, Color};

fn lines(text: &str, width: usize, fg: Color) -> Vec<CardLine> {
    crate::chatmd::map_lines(crate::md::render_chat(text, width), width, fg)
}

fn row_text(line: &CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}

/// A table's header row reads in the accent, bold; the rule and the column
/// separators are the muted grid, one colour where `│` meets `┼`; the body
/// rows keep the card colour.
#[test]
fn a_table_header_takes_the_accent_over_a_muted_rule() {
    let _a = crate::palette::test_guard();
    let _guard = crate::app::theme_test_guard();
    let fg = (9, 9, 9);
    let out = lines("| name | n |\n|---|---|\n| **a** | 1 |", 40, fg);
    assert_eq!(row_text(&out[0]), " name │ n");
    let head = &out[0][1];
    assert_eq!(head.fg, crate::palette::accent());
    assert!(head.bold);
    assert_eq!(out[0][6].c, '\u{2502}');
    let muted = crew_theme::theme().text_muted;
    assert_eq!(
        out[0][6].fg, muted,
        "the separator is the grid, not the header"
    );
    assert_eq!(out[1][1].c, '\u{2500}');
    assert_eq!(out[1][1].fg, muted);
    let cross = out[1]
        .iter()
        .position(|c| c.c == '\u{253c}')
        .expect("a crossing");
    assert_eq!(
        out[1][cross].fg, out[2][cross].fg,
        "┼ meets │ in one colour"
    );
    assert_eq!(out[2][1].fg, fg, "a bold body cell keeps the card colour");
    assert!(out[2][1].bold);
}

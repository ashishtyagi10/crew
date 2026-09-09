use super::*;
use crate::chatbody::plain;
use crew_lsp::{Position, Range, Severity};

fn diag(l0: u32, c0: u32, l1: u32, c1: u32, sev: Severity) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position {
                line: l0,
                character: c0,
            },
            end: Position {
                line: l1,
                character: c1,
            },
        },
        severity: sev,
        message: String::new(),
        source: None,
    }
}

#[test]
fn offsets_count_expanded_tabs_and_dropped_returns() {
    assert_eq!(rendered_offset("let x", 4, false), 4);
    // A tab at column 0 is eight spaces on screen.
    assert_eq!(rendered_offset("\tlet x", 1, false), 8);
    assert_eq!(rendered_offset("\tlet x", 5, false), 12);
    // `ab\tc`: the tab after two chars widens to the next stop (6 spaces).
    assert_eq!(rendered_offset("ab\tc", 3, false), 8);
    // A CR is drawn only when revealed.
    assert_eq!(rendered_offset("x\r", 2, false), 1);
    assert_eq!(rendered_offset("x\r", 2, true), 2);
    // UTF-16: an astral char is two units on the wire, one char on screen.
    assert_eq!(rendered_offset("\u{1F600}x", 2, false), 1);
    // Past the end clamps to the line's length.
    assert_eq!(rendered_offset("abc", 99, false), 3);
}

#[test]
fn spans_cover_each_line_of_a_range_and_widen_an_empty_one() {
    let _g = crate::app::theme_test_guard();
    let text = "fn main() {\n    let x = 1;\n    x\n}\n";
    let s = spans(
        text,
        &[
            diag(1, 8, 1, 9, Severity::Error),
            diag(1, 4, 2, 5, Severity::Warning),
            diag(3, 0, 3, 0, Severity::Error),
            diag(0, 0, 0, 2, Severity::Hint),
        ],
        false,
    );
    let bell = crew_theme::theme().bell;
    assert_eq!(
        s[0],
        Span {
            line: 1,
            start: 8,
            end: 9,
            color: bell
        }
    );
    assert_eq!((s[1].line, s[1].start, s[1].end), (1, 4, 14));
    assert_eq!((s[2].line, s[2].start, s[2].end), (2, 0, 5));
    assert_eq!(
        (s[3].line, s[3].start, s[3].end),
        (3, 0, 1),
        "an empty range is one char"
    );
    assert_eq!(s.len(), 4, "a hint draws no underline");
}

/// Rows as the viewer lays them: number gutter, then text, one cell per
/// char; and the screen cells the renderer would place for them.
fn fixture(rows: &[&str]) -> (Vec<CardLine>, Vec<CellView>) {
    let ink = crew_theme::theme().ink;
    let lines: Vec<CardLine> = rows
        .iter()
        .map(|r| r.chars().map(|c| plain(c, ink, false)).collect())
        .collect();
    let mut out = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        let mut col = 0u16;
        for cell in line {
            out.push(CellView {
                col,
                row: row as u16,
                c: cell.c,
                ..Default::default()
            });
            col += crate::chatwidth::char_w(cell.c) as u16;
        }
    }
    (lines, out)
}

fn underlined(out: &[CellView], row: u16) -> String {
    out.iter()
        .filter(|c| c.row == row && c.deco.line == DecoLine::Curly)
        .map(|c| c.c)
        .collect()
}

#[test]
fn the_underline_lands_under_the_range_on_the_right_row() {
    let _g = crate::app::theme_test_guard();
    let text = "fn main() {\n    let x: i32 = \"s\";\n}\n";
    let (lines, mut out) = fixture(&[
        "    1 fn main() {",
        "    2     let x: i32 = \"s\";",
        "    3 }",
    ]);
    let s = spans(text, &[diag(1, 17, 1, 20, Severity::Error)], false);
    apply(&mut out, &lines, 0, 3, 0, &s);
    assert_eq!(underlined(&out, 1), "\"s\"");
    assert_eq!(underlined(&out, 0), "");
    assert_eq!(underlined(&out, 2), "");
    let cell = out.iter().find(|c| c.deco.line == DecoLine::Curly).unwrap();
    assert_eq!(cell.deco.color, Some(crew_theme::theme().bell));
}

/// A span that runs onto a wrap continuation is drawn across both rows —
/// and when the window starts ON the continuation, the offset is recovered
/// by walking back to the numbered row.
#[test]
fn a_wrapped_line_keeps_its_offsets_across_rows_and_from_a_scrolled_top() {
    let _g = crate::app::theme_test_guard();
    let text = "abcdefghij\n";
    let (lines, mut out) = fixture(&["    1 abcde", "    \u{21aa} fghij"]);
    let s = spans(text, &[diag(0, 3, 0, 8, Severity::Warning)], false);
    apply(&mut out, &lines, 0, 2, 0, &s);
    assert_eq!(underlined(&out, 0), "de");
    assert_eq!(underlined(&out, 1), "fgh");
    // Scrolled so the continuation is the first visible row.
    let (lines, mut out) = fixture(&["    1 abcde", "    \u{21aa} fghij"]);
    let mut shown: Vec<CellView> = out.drain(..).filter(|c| c.row == 1).collect();
    for c in &mut shown {
        c.row = 0;
    }
    apply(&mut shown, &lines, 1, 1, 0, &s);
    assert_eq!(underlined(&shown, 0), "fgh");
}

/// With margins prepended (blame + the mark column), the text starts
/// further right; a banner row is skipped, not misread as a line.
#[test]
fn margins_shift_the_text_column_and_banners_are_left_alone() {
    let _g = crate::app::theme_test_guard();
    let text = "xyz\n";
    let (lines, mut out) = fixture(&["showing first 1 of 2", "\u{25cf} aaa     1 xyz"]);
    let s = spans(text, &[diag(0, 1, 0, 3, Severity::Error)], false);
    apply(&mut out, &lines, 0, 2, 6, &s);
    assert_eq!(underlined(&out, 0), "");
    assert_eq!(underlined(&out, 1), "yz");
}

#[test]
fn nothing_to_draw_is_a_no_op() {
    let (lines, mut out) = fixture(&["    1 x"]);
    apply(&mut out, &lines, 0, 1, 0, &[]);
    apply(
        &mut out,
        &lines,
        5,
        1,
        0,
        &[Span {
            line: 0,
            start: 0,
            end: 1,
            color: (1, 2, 3),
        }],
    );
    assert!(out.iter().all(|c| c.deco.line == DecoLine::None));
}

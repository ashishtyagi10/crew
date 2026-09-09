use super::*;
use crate::chatbody::plain;
use crew_lsp::{Position, Range};

fn diag(line: u32, sev: Severity) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 1 },
        },
        severity: sev,
        message: String::new(),
        source: None,
    }
}

fn cardline(s: &str) -> CardLine {
    s.chars()
        .map(|c| plain(c, crew_theme::theme().ink, false))
        .collect()
}

fn text_of(line: &CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}

#[test]
fn an_error_outranks_a_warning_on_the_same_line_and_hints_earn_nothing() {
    let _g = crate::app::theme_test_guard();
    let m = marks(&[
        diag(0, Severity::Warning),
        diag(0, Severity::Error),
        diag(2, Severity::Warning),
        diag(3, Severity::Hint),
    ]);
    assert_eq!(m.len(), 4);
    assert_eq!(m[0].map(|(c, _)| c), Some('\u{25cf}'));
    assert_eq!(m[1], None);
    assert_eq!(m[2].map(|(c, _)| c), Some('\u{25b2}'));
    assert_eq!(m[3], None);
    assert_eq!(
        m[0].unwrap().1,
        crew_theme::theme().bell,
        "errors wear the bell ink"
    );
    assert_eq!(m[2].unwrap().1, crew_theme::theme().status_fg);
    assert!(marks(&[]).is_empty());
}

#[test]
fn the_margin_marks_the_row_a_line_starts_and_blanks_its_continuations() {
    let _g = crate::app::theme_test_guard();
    let mut lines = vec![
        cardline("    1 fn main() {"),
        cardline("    2     let x: i32 = \"s\";"),
        cardline("    \u{21aa} wrapped"),
        cardline("    3 }"),
    ];
    let m = marks(&[diag(1, Severity::Error)]);
    apply(&mut lines, &m, 0);
    assert_eq!(text_of(&lines[0]), "      1 fn main() {");
    assert_eq!(text_of(&lines[1]), "\u{25cf}     2     let x: i32 = \"s\";");
    assert_eq!(text_of(&lines[2]), "      \u{21aa} wrapped");
    assert_eq!(text_of(&lines[3]), "      3 }");
    assert_eq!(lines[1][0].fg, crew_theme::theme().bell);
    assert!(lines[1][0].bold);
    for l in &lines {
        assert_eq!(
            l.len(),
            text_of(l).chars().count(),
            "every row grew by exactly the margin"
        );
    }
}

/// With a blame column already prepended, the number gutter is `at` cells
/// in; the mark still lands on the right row.
#[test]
fn the_margin_reads_the_number_past_a_blame_column() {
    let _g = crate::app::theme_test_guard();
    let mut lines = vec![
        cardline("aaa1111 Ada    1 fn main() {"),
        cardline("bbb2222 Bob    2 }"),
    ];
    apply(&mut lines, &marks(&[diag(1, Severity::Warning)]), 11);
    assert!(
        text_of(&lines[0]).starts_with("  aaa1111"),
        "{}",
        text_of(&lines[0])
    );
    assert!(
        text_of(&lines[1]).starts_with("\u{25b2} bbb2222"),
        "{}",
        text_of(&lines[1])
    );
}

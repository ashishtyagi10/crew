use super::*;
use crew_lsp::{Position, Range};

fn diag(line: u32, severity: Severity, message: &str) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 4 },
            end: Position { line, character: 9 },
        },
        severity,
        message: message.into(),
        source: Some("rust-analyzer".into()),
    }
}

/// An error outranks a warning on the same line, and the others are counted
/// rather than dropped — the row has one line and three things to say.
#[test]
fn the_worst_diagnostic_speaks_and_the_rest_are_counted() {
    let _g = crate::app::theme_test_guard();
    let d = vec![
        diag(2, Severity::Warning, "unused variable: `arc`"),
        diag(2, Severity::Error, "cannot find function `helper`"),
        diag(2, Severity::Warning, "unused import"),
    ];
    let (text, _) = note(&d, 2).expect("a note");
    assert_eq!(text, "cannot find function `helper`  +2");
    assert_eq!(note(&d, 3), None, "a line with nothing on it");
}

/// A server's message may be a paragraph; the row takes its first line.
#[test]
fn a_paragraph_is_cut_to_its_first_line() {
    let _g = crate::app::theme_test_guard();
    let d = vec![diag(
        0,
        Severity::Error,
        "mismatched types\nexpected `f32`, found `usize`",
    )];
    assert_eq!(note(&d, 0).unwrap().0, "mismatched types");
}

/// Information and hints are not marked in the margin, so they do not speak
/// here either — they would put a note on every unused import.
#[test]
fn only_errors_and_warnings_say_anything() {
    let _g = crate::app::theme_test_guard();
    let d = vec![diag(1, Severity::Information, "consider importing this")];
    assert_eq!(note(&d, 1), None);
}

/// The note is the severity's colour taken toward the page — quieter than the
/// margin's mark, and still nothing like the page itself.
#[test]
fn the_note_is_quieter_than_the_mark_but_not_the_page() {
    let _g = crate::app::theme_test_guard();
    let d = vec![diag(0, Severity::Error, "boom")];
    let (_, fg) = note(&d, 0).unwrap();
    let mark = crate::viewpane::lspgutter::mark_for(Severity::Error)
        .unwrap()
        .1;
    let bg = crew_theme::theme().page_bg;
    let far = |a: (u8, u8, u8), b: (u8, u8, u8)| {
        (a.0 as i32 - b.0 as i32).abs()
            + (a.1 as i32 - b.1 as i32).abs()
            + (a.2 as i32 - b.2 as i32).abs()
    };
    assert!(far(fg, bg) > 120, "the note vanished into the page: {fg:?}");
    assert!(far(fg, mark) > 0, "the note is the mark's own colour");
}

/// The note goes UNDER the line, on a row of its own, and the rows below it
/// keep their order — the search marks that pointed at them move down too.
#[test]
fn the_note_takes_a_row_under_its_line_and_marks_follow() {
    let _g = crate::app::theme_test_guard();
    let text = "fn a() {}\nfn b() {}\nfn c() {}\n";
    let mut lines = crate::viewpane::linepaint::numbered(
        text,
        40,
        "rust",
        (200, 200, 200),
        (100, 100, 100),
        &[],
    );
    let mut marks = vec![
        Mark {
            row: 0,
            label: "a".into(),
            depth: 0,
        },
        Mark {
            row: 2,
            label: "c".into(),
            depth: 0,
        },
    ];
    let before = lines.len();
    insert(
        &mut lines,
        &mut marks,
        &[diag(1, Severity::Error, "boom")],
        40,
    );
    assert_eq!(lines.len(), before + 1, "one note row");
    let text_of = |l: &CardLine| l.iter().map(|c| c.c).collect::<String>();
    assert!(
        text_of(&lines[2]).contains("\u{2191} boom"),
        "{:?}",
        text_of(&lines[2])
    );
    assert!(
        text_of(&lines[3]).contains("fn c"),
        "the next line moved down"
    );
    // The mark above the insertion stays; the one below moves with its row.
    assert_eq!(marks[0].row, 0);
    assert_eq!(marks[1].row, 3);
}

/// A note row is not a source row: the walk that lays underlines down must
/// not read it as one, or the line under a diagnostic gets the next line's
/// squiggle.
#[test]
fn a_note_row_is_not_a_source_row() {
    let _g = crate::app::theme_test_guard();
    let mut lines = crate::viewpane::linepaint::numbered(
        "fn a() {}\nfn b() {}\n",
        40,
        "rust",
        (200, 200, 200),
        (100, 100, 100),
        &[],
    );
    let mut marks: Vec<Mark> = Vec::new();
    insert(
        &mut lines,
        &mut marks,
        &[diag(0, Severity::Warning, "hmm")],
        40,
    );
    assert!(
        matches!(kind(&lines[1], 0), Row::Other),
        "the note row claims a source line"
    );
}

/// Nothing to say, nothing inserted — a file with a clean bill is the same
/// rows it was.
#[test]
fn a_clean_file_gains_no_rows() {
    let _g = crate::app::theme_test_guard();
    let mut lines = crate::viewpane::linepaint::numbered(
        "fn a() {}\n",
        40,
        "rust",
        (200, 200, 200),
        (100, 100, 100),
        &[],
    );
    let before = lines.len();
    insert(&mut lines, &mut [], &[], 40);
    assert_eq!(lines.len(), before);
}

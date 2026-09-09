//! The frame's one line, read state by state.
use super::legend;
use crate::viewpane::caret::Step;
use crate::viewpane::editpane_tests::doc;
use crew_term::GridSize;

const TALL: GridSize = GridSize { cols: 60, rows: 40 };
const SHORT: GridSize = GridSize { cols: 60, rows: 4 };

#[test]
fn a_viewer_says_the_file_and_an_editor_adds_where_the_caret_is() {
    let mut p = doc();
    assert_eq!(legend(&p, TALL, None, None), "caret.md");
    p.start_editing(TALL.cols);
    assert_eq!(legend(&p, TALL, None, None), "caret.md \u{b7} 1:3");
    p.move_caret(Step::Bottom, TALL.cols, TALL.rows);
    assert_eq!(legend(&p, TALL, None, None), "caret.md \u{b7} 10:41");
}

#[test]
fn the_position_sits_between_the_name_and_the_progress() {
    let mut p = doc();
    p.start_editing(SHORT.cols);
    let l = legend(&p, SHORT, None, None);
    assert!(l.starts_with("caret.md \u{b7} 1:3 \u{b7} "), "{l}");
    assert!(l.ends_with('%'), "{l}");
    p.move_caret(Step::Bottom, SHORT.cols, SHORT.rows);
    assert_eq!(
        legend(&p, SHORT, None, None),
        "caret.md \u{b7} 10:41 \u{b7} 100%"
    );
}

#[test]
fn the_dirty_dot_stays_on_the_name_and_a_hint_takes_the_rest() {
    let mut p = doc();
    p.start_editing(TALL.cols);
    p.insert("x", TALL.cols, TALL.rows);
    assert_eq!(legend(&p, TALL, None, None), "caret.md \u{25cf} \u{b7} 1:4");
    assert_eq!(
        legend(&p, TALL, Some("saved"), None),
        "caret.md \u{25cf} \u{b7} saved"
    );
    assert_eq!(
        legend(&p, TALL, None, Some("url: ".into())),
        "caret.md \u{25cf} \u{b7} url: ",
        "an open field has the line"
    );
}

/// What the language server said sits between the position and the
/// progress — and while it is still starting, the legend says that.
#[test]
fn the_language_servers_word_sits_after_the_position() {
    use crate::viewpane::lspjob::Lsp;
    use crew_lsp::{Diagnostic, Position, Range, Severity};
    let diag = |sev| Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 1,
            },
        },
        severity: sev,
        message: String::new(),
        source: None,
    };
    let mut p = doc();
    p.lsp = Lsp::On(vec![
        diag(Severity::Error),
        diag(Severity::Error),
        diag(Severity::Warning),
    ]);
    assert_eq!(
        legend(&p, TALL, None, None),
        "caret.md \u{b7} 2 errors \u{b7} 1 warning"
    );
    p.start_editing(TALL.cols);
    assert_eq!(
        legend(&p, TALL, None, None),
        "caret.md \u{b7} 1:3 \u{b7} 2 errors \u{b7} 1 warning"
    );
    let (_tx, rx) = std::sync::mpsc::channel();
    p.lsp = Lsp::Loading {
        rx,
        server: "rust-analyzer".into(),
    };
    assert_eq!(
        legend(&p, TALL, None, None),
        "caret.md \u{b7} 1:3 \u{b7} lsp: rust-analyzer starting\u{2026}"
    );
    p.lsp = Lsp::Skipped;
    assert_eq!(legend(&p, TALL, None, None), "caret.md \u{b7} 1:3");
}

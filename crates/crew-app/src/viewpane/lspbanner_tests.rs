//! The banner over the file — what the server said about the WHOLE of it.
//! Split from `lspmsg_tests` (the per-line notes) for the line cap.
use super::super::lspjob::Lsp;
use super::*;
use crate::chatbody::CardLine;
use crew_lsp::{Diagnostic, Position, Range, Severity};

fn diag(line: u32, severity: Severity, message: &str) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 4 },
        },
        severity,
        message: message.into(),
        source: None,
    }
}

/// A pane with marks in its margin says how many, over the file — the count
/// the document window's legend has always shown and a `/view` tile never
/// could.
#[test]
fn the_banner_says_what_the_server_found() {
    let _g = crate::app::theme_test_guard();
    let text = |l: &CardLine| l.iter().map(|c| c.c).collect::<String>();
    let on = Lsp::On(vec![
        diag(1, Severity::Error, "boom"),
        diag(2, Severity::Warning, "hmm"),
    ]);
    let b = top_banner(&on, 40).expect("a banner");
    assert!(text(&b).contains("1 error"), "{:?}", text(&b));
    assert!(text(&b).contains("1 warning"), "{:?}", text(&b));
}

/// A margin that never appears has to be able to explain itself: a server
/// still starting, and one that failed, both say so.
#[test]
fn a_missing_margin_explains_itself() {
    let _g = crate::app::theme_test_guard();
    let text = |l: &CardLine| l.iter().map(|c| c.c).collect::<String>();
    let failed = Lsp::Failed("rust-analyzer not installed".into());
    let b = top_banner(&failed, 60).expect("a banner");
    assert!(text(&b).contains("not installed"), "{:?}", text(&b));
}

/// A clean file spends no row on saying so: the absence of marks is the
/// answer, and the pane's rows are the file's.
#[test]
fn a_clean_file_gets_no_banner() {
    let _g = crate::app::theme_test_guard();
    let clean = Lsp::On(Vec::new());
    assert!(top_banner(&clean, 40).is_none());
    let hints = Lsp::On(vec![diag(
        0,
        Severity::Information,
        "consider importing this",
    )]);
    assert!(top_banner(&hints, 40).is_none(), "hints mark nothing");
    assert!(top_banner(&Lsp::Off, 40).is_none());
}

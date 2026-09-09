use super::*;
use crew_lsp::{Position, Range, Severity};

fn diag(line: u32, sev: Severity) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 3 },
        },
        severity: sev,
        message: "m".into(),
        source: None,
    }
}

#[test]
fn the_summary_counts_errors_and_warnings_only() {
    assert_eq!(summary(&[]), "no diagnostics");
    assert_eq!(summary(&[diag(1, Severity::Error)]), "1 error");
    assert_eq!(
        summary(&[diag(1, Severity::Warning), diag(2, Severity::Warning)]),
        "2 warnings"
    );
    assert_eq!(
        summary(&[
            diag(1, Severity::Error),
            diag(2, Severity::Error),
            diag(3, Severity::Warning),
            diag(4, Severity::Hint),
            diag(5, Severity::Information),
        ]),
        "2 errors \u{b7} 1 warning"
    );
}

#[test]
fn status_speaks_while_loading_and_once_on_and_otherwise_holds_its_tongue() {
    assert_eq!(Lsp::Off.status(), None);
    assert_eq!(Lsp::Skipped.status(), None);
    assert_eq!(Lsp::Failed("x".into()).status().as_deref(), Some("lsp: x"));
    let (_tx, rx) = mpsc::channel();
    let loading = Lsp::Loading {
        rx,
        server: "rust-analyzer".into(),
    };
    assert_eq!(
        loading.status().as_deref(),
        Some("lsp: rust-analyzer starting\u{2026}")
    );
    assert_eq!(
        Lsp::On(vec![diag(0, Severity::Error)]).status().as_deref(),
        Some("1 error")
    );
    // An empty answer draws nothing but still says so on the legend.
    assert!(Lsp::On(vec![]).diags().is_none());
    assert_eq!(Lsp::On(vec![]).status().as_deref(), Some("no diagnostics"));
}

#[test]
fn poll_settles_on_an_answer_and_on_a_worker_that_died() {
    let (tx, rx) = mpsc::channel();
    let mut l = Lsp::Loading {
        rx,
        server: "s".into(),
    };
    assert!(!l.poll(), "nothing yet");
    tx.send(Ok(vec![diag(0, Severity::Warning)])).unwrap();
    assert!(l.poll());
    assert_eq!(l.diags().map(<[_]>::len), Some(1));
    assert!(!l.poll(), "settled states are quiet");

    let (tx, rx) = mpsc::channel::<Result<Vec<Diagnostic>, String>>();
    let mut l = Lsp::Loading {
        rx,
        server: "s".into(),
    };
    drop(tx);
    assert!(l.poll());
    assert!(matches!(l, Lsp::Failed(_)));
}

/// The pane asks only once the file is a CODE file that has landed, and
/// under test the switch is off, so it declines rather than spawning.
#[test]
fn a_pane_declines_without_the_switch_and_never_asks_about_prose() {
    use crate::viewpane::load::Loaded;
    let mut p = ViewPane::open(std::env::temp_dir().join("lspjob.rs"));
    assert!(!p.poll_lsp(), "still loading: nothing to ask about");
    assert!(matches!(p.lsp, Lsp::Off));
    p.state = LoadState::Ready {
        format: Format::Code { lang: "rust" },
        loaded: Loaded {
            text: "fn main() {}\n".into(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    assert!(!p.poll_lsp());
    assert!(matches!(p.lsp, Lsp::Skipped), "never started under test");
    p.reload();
    assert!(matches!(p.lsp, Lsp::Off), "a re-read asks again");

    let mut m = ViewPane::open(std::env::temp_dir().join("notes.md"));
    m.state = LoadState::Ready {
        format: Format::Markdown,
        loaded: Loaded {
            text: "# hi\n".into(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    assert!(!m.poll_lsp());
    assert!(
        matches!(m.lsp, Lsp::Off),
        "prose is never a language server's business"
    );
}

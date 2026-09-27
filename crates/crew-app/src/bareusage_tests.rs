//! A command typed with nothing after it answers with ITS usage line.
//!
//! Bare `/md` said `usage: /view <path>` — but `/md <path>` opens a document
//! window, not the viewer — and bare `/find` fell through to the typo check,
//! which said `unknown command /find — did you mean /find?`.

use crate::app::CrewApp;

fn status_after(cmd: &str) -> String {
    let mut app = CrewApp::default();
    app.submit_input(cmd.to_string());
    app.status
        .as_ref()
        .map(|(m, _)| m.clone())
        .unwrap_or_default()
}

#[test]
fn bare_md_names_md() {
    assert_eq!(status_after("/md"), "usage: /md <path>");
    assert_eq!(status_after("/view"), "usage: /view <path>");
}

#[test]
fn bare_find_is_a_usage_line_not_a_typo() {
    for cmd in ["/find", "/find "] {
        let msg = status_after(cmd);
        assert!(msg.starts_with("usage: /find <text>"), "{cmd:?}: {msg:?}");
        assert!(!msg.contains("did you mean"), "{cmd:?}: {msg:?}");
    }
}

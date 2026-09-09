use super::*;

/// A load earns a LOG line said the way the pane's line says it — and the
/// verb is the kind's: a skill is applied, a server connected, a language
/// server started.
#[test]
fn a_load_tees_one_lifecycle_line_with_its_kinds_verb() {
    let ev = |kind: &str, name: &str| HiveEvent::Loaded {
        agent: String::new(),
        kind: kind.into(),
        name: name.into(),
        detail: "whatever".into(),
    };
    assert_eq!(
        log_line(None, &ev("skill", "rust-testing")),
        Some((false, "smith: skill rust-testing applied".into()))
    );
    assert_eq!(
        log_line(None, &ev("mcp", "github")),
        Some((false, "smith: mcp github connected".into()))
    );
    assert_eq!(
        log_line(None, &ev("lsp", "rust-analyzer")),
        Some((false, "smith: lsp rust-analyzer started".into()))
    );
}

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

/// A relay agent's own tool call — its id minted from its name, as the
/// broker and the pane both mint it — is logged by name; a hive agent
/// nobody named keeps its number.
#[test]
fn a_named_agents_tool_call_is_logged_by_name() {
    let names = std::collections::HashMap::from([(
        crew_hive::AgentId::minted("claude").0,
        "claude".to_string(),
    )]);
    let call = |id: crew_hive::AgentId| HiveEvent::ToolCall {
        agent: id,
        label: "Read".into(),
        args: String::new(),
    };
    assert_eq!(
        log_line_named(None, &names, &call(crew_hive::AgentId::minted("claude"))),
        Some((false, "smith: claude called Read".into()))
    );
    assert_eq!(
        log_line_named(None, &names, &call(crew_hive::AgentId(3))),
        Some((false, "smith: agent 3 called Read".into()))
    );
    // A roster pre-binds the same ids the broker will send.
    let mut tools = crate::chattool::ToolLines::default();
    tools.prebind(["claude", "codex"].into_iter());
    assert_eq!(
        tools
            .names
            .get(&crew_hive::AgentId::minted("codex").0)
            .map(String::as_str),
        Some("codex")
    );
}

/// A playbook the MODEL chose is logged as chosen; one the task named, as
/// applied — the verb the pane's own `Loaded` line leads its detail with.
#[test]
fn a_skill_the_model_chose_is_logged_as_chosen_and_a_named_one_as_applied() {
    let ev = |detail: &str| HiveEvent::Loaded {
        agent: String::new(),
        kind: "skill".into(),
        name: "code-review".into(),
        detail: detail.into(),
    };
    assert_eq!(
        log_line(None, &ev("chose \u{b7} reads a diff for bugs")),
        Some((false, "smith: skill code-review chosen".into()))
    );
    assert_eq!(
        log_line(None, &ev("applied \u{b7} reads a diff for bugs")),
        Some((false, "smith: skill code-review applied".into()))
    );
    // Only the leading word decides: a description that mentions choosing
    // does not.
    assert_eq!(
        log_line(None, &ev("applied \u{b7} chose wisely")),
        Some((false, "smith: skill code-review applied".into()))
    );
}

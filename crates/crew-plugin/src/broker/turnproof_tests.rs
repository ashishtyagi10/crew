use crew_hive::{AgentId, HiveEvent};

fn call(label: &str, args: &str) -> HiveEvent {
    HiveEvent::ToolCall {
        agent: AgentId::minted("engineer"),
        label: label.into(),
        args: args.into(),
    }
}

fn result(label: &str, ok: bool, text: &str) -> HiveEvent {
    HiveEvent::ToolResult {
        agent: AgentId::minted("engineer"),
        label: label.into(),
        ok,
        text: text.into(),
        ms: 3,
    }
}

/// One test, because the log is the process's: two tests filling it at once
/// would read each other's lines.
#[test]
fn a_turn_reads_as_its_calls_and_how_each_came_back() {
    super::start();
    super::note(&call("sys:edit", "{\"path\": \"a.rs\"}"));
    super::note(&call("sys:run", "{\"cmd\": \"cargo test\"}"));
    super::note(&result(
        "sys:edit",
        true,
        "edited a.rs at line 4 (1 line)\n4│ x",
    ));
    super::note(&result("sys:run", false, "\nexit 101\n--- stderr ---"));
    assert_eq!(
        super::lines(),
        "1. sys:edit {\"path\": \"a.rs\"} \u{2192} ok: edited a.rs at line 4 (1 line)\n\
         2. sys:run {\"cmd\": \"cargo test\"} \u{2192} failed: exit 101"
    );
    super::start();
    assert_eq!(super::lines(), "", "a verified turn starts from nothing");
    for i in 0..super::KEEP + 5 {
        super::note(&call("sys:grep", &i.to_string()));
    }
    let kept = super::lines();
    assert_eq!(kept.lines().count(), super::KEEP);
    assert!(kept.starts_with("1. sys:grep 5"), "the oldest go first");
    super::start();
}

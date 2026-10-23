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

/// A log of the test's own: the process's is noted into by the broker's
/// other tests at the same time.
#[test]
fn a_turn_reads_as_its_calls_and_how_each_came_back() {
    let mut log = super::Log::default();
    log.note(&call("sys:edit", "{\"path\": \"a.rs\"}"));
    log.note(&call("sys:run", "{\"cmd\": \"cargo test\"}"));
    log.note(&result(
        "sys:edit",
        true,
        "edited a.rs at line 4 (1 line)\n4\u{2502} x",
    ));
    log.note(&result("sys:run", false, "\nexit 101\n--- stderr ---"));
    assert_eq!(
        log.lines(),
        "1. sys:edit {\"path\": \"a.rs\"} \u{2192} ok: edited a.rs at line 4 (1 line)\n\
         2. sys:run {\"cmd\": \"cargo test\"} \u{2192} failed: exit 101"
    );
    let mut log = super::Log::default();
    assert_eq!(log.lines(), "", "a verified turn starts from nothing");
    for i in 0..super::KEEP + 5 {
        log.note(&call("sys:grep", &i.to_string()));
    }
    let kept = log.lines();
    assert_eq!(kept.lines().count(), super::KEEP);
    assert!(kept.starts_with("1. sys:grep 5"), "the oldest go first");
}

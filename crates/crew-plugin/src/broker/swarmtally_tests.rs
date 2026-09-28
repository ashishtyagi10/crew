use super::*;

fn budget(used: u32, total: u32) -> HiveEvent {
    HiveEvent::ToolBudget { used, total }
}

fn refused() -> HiveEvent {
    HiveEvent::ToolResult {
        agent: crew_hive::AgentId(2),
        label: "weather:current".into(),
        ok: false,
        text: "not run \u{2014} tool budget spent (12 calls this run)".into(),
        ms: 0,
    }
}

/// The refusal can reach the bus before the draw that emptied the pool: the
/// note rides whichever comes first, and is said once.
#[test]
fn a_refusal_that_beats_the_empty_reading_still_follows_the_note() {
    let mut t = Tally::new(3);
    assert!(t.observe(&budget(11, 12)).is_none());
    let note = t.observe(&refused()).expect("the refusal says it");
    assert!(
        matches!(note, PluginEvent::Message { ref text, .. } if text.starts_with("tool budget spent \u{2014} 12 calls"))
    );
    assert!(t.observe(&budget(12, 12)).is_none(), "said once");
    assert!(t.observe(&refused()).is_none());
}

#[test]
fn the_empty_reading_says_it_when_it_comes_first() {
    let mut t = Tally::new(3);
    assert!(t.observe(&budget(12, 12)).is_some());
    assert!(t.observe(&refused()).is_none(), "said once");
}

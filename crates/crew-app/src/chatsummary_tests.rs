use super::*;
use std::collections::HashMap;

fn agent(name: &str, model: &str) -> AgentInfo {
    AgentInfo {
        name: name.into(),
        role: String::new(),
        model: model.into(),
    }
}

fn ctx(pairs: &[(&str, u64)]) -> HashMap<String, u64> {
    pairs.iter().map(|(n, v)| (n.to_string(), *v)).collect()
}

#[test]
fn nothing_to_summarise_is_none() {
    // No agents and no spend → no footer at all.
    assert_eq!(summary_text(&[], &HashMap::new(), 0), None);
}

#[test]
fn tokens_alone_still_summarise() {
    // Even with no roster, a nonzero session spend earns the footer.
    let s = summary_text(&[], &HashMap::new(), 9_500).unwrap();
    assert_eq!(s, "~9.5k tok");
}

#[test]
fn single_agent_shows_model_and_full_context() {
    // A fresh agent (no ctx recorded) reads as a full window: 100% remaining.
    let agents = [agent("planner", "anthropic/claude-sonnet-5")];
    let s = summary_text(&agents, &HashMap::new(), 0).unwrap();
    assert_eq!(s, "claude-sonnet-5 \u{00b7} 100% context");
}

#[test]
fn context_derives_from_fill_over_the_model_limit() {
    // claude → 200k window; 100k used → 50% fill → 50% remaining.
    let agents = [agent("planner", "claude")];
    let s = summary_text(&agents, &ctx(&[("planner", 100_000)]), 0).unwrap();
    assert!(s.contains("50% context"), "got: {s}");
}

#[test]
fn session_tokens_append_once_nonzero() {
    let agents = [agent("planner", "claude")];
    let s = summary_text(&agents, &HashMap::new(), 12_000).unwrap();
    assert!(s.ends_with("~12.0k tok"), "token spend trails: {s}");
    assert!(s.starts_with("claude"), "model leads: {s}");
}

#[test]
fn shared_model_shows_once_distinct_models_collapse_to_a_count() {
    let same = [agent("a", "claude"), agent("b", "claude")];
    assert!(
        summary_text(&same, &HashMap::new(), 0)
            .unwrap()
            .starts_with("claude"),
        "one shared model shows by name"
    );
    let diff = [agent("a", "claude"), agent("b", "gpt-5")];
    assert!(
        summary_text(&diff, &HashMap::new(), 0)
            .unwrap()
            .starts_with("2 agents"),
        "mixed models collapse to an agent count"
    );
}

#[test]
fn tightest_remaining_context_wins() {
    // planner at 50% fill (100k/200k), coder at 90% fill (180k/200k). The
    // footer must report the agent NEAREST its ceiling: 10% remaining.
    let agents = [agent("planner", "claude"), agent("coder", "claude")];
    let s = summary_text(
        &agents,
        &ctx(&[("planner", 100_000), ("coder", 180_000)]),
        0,
    )
    .unwrap();
    assert!(s.contains("10% context"), "min remaining wins: {s}");
}

#[test]
fn unknown_model_limit_drops_the_context_segment() {
    // A model with no known window can't yield a percentage — the context
    // segment is omitted rather than guessed.
    let agents = [agent("planner", "some-exotic-model")];
    let s = summary_text(&agents, &ctx(&[("planner", 5_000)]), 3_000).unwrap();
    assert!(!s.contains("context"), "no ctx% without a known limit: {s}");
    assert!(s.contains("~3.0k tok"), "other segments still show: {s}");
}

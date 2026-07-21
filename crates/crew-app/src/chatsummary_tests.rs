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

fn stats(pairs: &[(&str, u32, u64)]) -> HashMap<String, (u32, u64)> {
    pairs
        .iter()
        .map(|(n, r, ms)| (n.to_string(), (*r, *ms)))
        .collect()
}

#[test]
fn block_shows_every_stat_row_with_data() {
    let agents = [agent("smith", "anthropic/claude-opus-4-8")];
    let rows = summary_block(
        &agents,
        &ctx(&[("smith", 100_000)]),
        120_000,
        8,
        &stats(&[("smith", 5, 21_000)]), // 5 replies, 21s total → avg 4.2s
    );
    let labels: Vec<&str> = rows.iter().map(|(l, _)| *l).collect();
    assert_eq!(labels, ["model", "ctx", "usage", "agents"]);
    let by = |k: &str| rows.iter().find(|(l, _)| *l == k).unwrap().1.clone();
    assert_eq!(by("model"), "claude-opus-4-8");
    // claude window is 200k; 100k used → 100k/200k · 50% left
    assert_eq!(by("ctx"), "100.0k/200.0k \u{00b7} 50% left");
    assert_eq!(by("usage"), "~120.0k tok \u{00b7} 8 turns");
    assert_eq!(by("agents"), "1 \u{00b7} avg 4.2s/reply");
}

#[test]
fn block_omits_rows_without_data() {
    // Tokens alone: no roster → no model/ctx/agents rows, only usage.
    let rows = summary_block(&[], &HashMap::new(), 9_500, 0, &HashMap::new());
    let labels: Vec<&str> = rows.iter().map(|(l, _)| *l).collect();
    assert_eq!(labels, ["usage"]);
    assert_eq!(rows[0].1, "~9.5k tok"); // no turns suffix at 0 turns
}

#[test]
fn block_mixed_models_collapse_to_a_count() {
    let agents = [agent("a", "claude"), agent("b", "gpt-5")];
    let rows = summary_block(&agents, &HashMap::new(), 0, 0, &HashMap::new());
    assert_eq!(rows[0], ("model", "mixed (2)".to_string()));
}

#[test]
fn block_agents_row_omits_latency_without_replies() {
    let agents = [agent("a", "claude"), agent("b", "claude")];
    let rows = summary_block(&agents, &HashMap::new(), 0, 0, &HashMap::new());
    let agents_row = rows.iter().find(|(l, _)| *l == "agents").unwrap();
    assert_eq!(agents_row.1, "2", "no reply stats → count only, no avg");
}

#[test]
fn block_lines_pad_labels_into_a_column() {
    let rows = vec![("model", "x".to_string()), ("agents", "y".to_string())];
    let lines = block_lines(&rows);
    // Widest label is "agents" (6); "model" pads to 6 so values align at col 7.
    assert_eq!(lines[0], "model  x");
    assert_eq!(lines[1], "agents y");
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

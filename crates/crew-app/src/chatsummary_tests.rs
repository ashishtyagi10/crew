use super::*;
use crew_plugin::AgentInfo;
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

fn text(line: &[(char, (u8, u8, u8))]) -> String {
    line.iter().map(|(c, _)| *c).collect()
}

fn fc<'a>(agents: &'a [AgentInfo], ctxm: &'a HashMap<String, u64>) -> FooterCtx<'a> {
    FooterCtx {
        agents,
        ctx: ctxm,
        tok_in: 41_600,
        tok_out: 314,
        cost_microusd: 129_000, // $0.129
        branch: Some("main"),
        input: "",
        windows: crate::usageledger::Windows {
            five_h: Some(crate::usageledger::WindowStat {
                left_ms: (3 * 60 + 52) * 60_000, // 3h52m
                spent: 150_000,
                budget: 5_000_000, // 3%
            }),
            seven_d: Some(crate::usageledger::WindowStat {
                left_ms: (3 * 24 + 23) * 3_600_000, // 3d23h
                spent: 0,
                budget: 25_000_000,
            }),
        },
    }
}

#[test]
fn line1_shows_model_branch_cost_and_split() {
    let agents = [agent("smith", "qwen/qwen3-coder-plus")];
    let lines = footer_lines(&fc(&agents, &ctx(&[("smith", 100_000)])), 120);
    assert_eq!(
        text(&lines[0]),
        "qwen3-coder-plus | main | $0.129 | 41.6k in / 314 out"
    );
}

#[test]
fn line1_hides_cost_when_unpriced_but_always_shows_tokens() {
    let empty_ctx = HashMap::new();
    let mut f = fc(&[], &empty_ctx);
    f.cost_microusd = 0;
    f.tok_in = 0;
    f.tok_out = 0;
    f.branch = None;
    let lines = footer_lines(&f, 120);
    assert_eq!(text(&lines[0]), "0 in / 0 out");
}

#[test]
fn line2_shows_countdowns_and_bars() {
    let agents = [agent("smith", "anthropic/claude-opus-4-8")];
    // opus limit 200k, 100k used → ctx bar 50%.
    let lines = footer_lines(&fc(&agents, &ctx(&[("smith", 100_000)])), 120);
    let l2 = text(&lines[1]);
    assert!(l2.starts_with("5h:3h52m | 7d:3d23h | "), "{l2}");
    assert!(l2.contains("3% (5h)"), "{l2}");
    assert!(l2.ends_with("50% (ctx)"), "{l2}");
}

#[test]
fn line2_dashes_when_no_window_and_drops_ctx_without_agents() {
    let empty_ctx = HashMap::new();
    let mut f = fc(&[], &empty_ctx);
    f.windows = crate::usageledger::Windows::default();
    let l2 = text(&footer_lines(&f, 120)[1]);
    assert_eq!(l2, "5h:-- | 7d:--");
}

#[test]
fn line2_drops_bars_on_narrow_panes() {
    let agents = [agent("smith", "anthropic/claude-opus-4-8")];
    let lines = footer_lines(&fc(&agents, &ctx(&[("smith", 100_000)])), 40);
    assert_eq!(text(&lines[1]), "5h:3h52m | 7d:3d23h");
}

#[test]
fn line3_swarm_by_default_relay_when_mentioning() {
    let agents = [agent("coder", "m")];
    let empty_ctx = HashMap::new();
    let f = fc(&agents, &empty_ctx);
    let l3 = text(&footer_lines(&f, 120)[2]);
    assert_eq!(
        l3,
        "\u{25b6}\u{25b6} swarm mode \u{00b7} / for constructs \u{00b7} @ to relay to an agent"
    );
    let mut f = fc(&agents, &empty_ctx);
    f.input = "@coder fix the tests";
    let l3 = text(&footer_lines(&f, 120)[2]);
    assert!(l3.starts_with("\u{25b6}\u{25b6} @coder relay"), "{l3}");
}

#[test]
fn segments_are_colored_separators_muted() {
    let agents = [agent("smith", "qwen3-coder-plus")];
    let lines = footer_lines(&fc(&agents, &ctx(&[])), 120);
    let th = crew_theme::theme();
    // First char of line 1 is the model segment → cyan (ansi[14]).
    assert_eq!(lines[0][0].1, th.ansi[14]);
    // The separator chars are muted.
    let sep = lines[0].iter().find(|(c, _)| *c == '|').unwrap();
    assert_eq!(sep.1, th.text_muted);
}

#[test]
fn mixed_roster_counts_models() {
    let agents = [agent("a", "m1"), agent("b", "m2")];
    let lines = footer_lines(&fc(&agents, &HashMap::new()), 120);
    assert!(text(&lines[0]).starts_with("2 agents | "));
}

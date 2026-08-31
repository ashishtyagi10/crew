use super::cost_microusd;

#[test]
fn longest_pattern_wins() {
    // qwen3-coder-flash must match its own cheaper rate, not qwen3-coder.
    // 1M in at $0.3/Mtok = 300_000 µ$.
    assert_eq!(cost_microusd("qwen3-coder-flash", 1_000_000, 0), 300_000);
    assert_eq!(cost_microusd("qwen3-coder-plus", 1_000_000, 0), 1_000_000);
}

#[test]
fn provider_prefix_and_case_are_ignored() {
    // $3/Mtok in + $15/Mtok out: 10k in + 1k out = 30_000 + 15_000 µ$.
    assert_eq!(
        cost_microusd("anthropic/Claude-Sonnet-5", 10_000, 1_000),
        45_000
    );
}

#[test]
fn unknown_model_costs_zero() {
    assert_eq!(cost_microusd("mock-model", 1_000_000, 1_000_000), 0);
    assert_eq!(cost_microusd("", 5, 5), 0);
}

#[test]
fn zero_tokens_cost_zero() {
    assert_eq!(cost_microusd("claude-opus-4-8", 0, 0), 0);
}

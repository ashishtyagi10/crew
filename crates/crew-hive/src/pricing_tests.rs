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

/// Alibaba Cloud Model Studio, International (Singapore), read 2026-09-28:
/// qwen-flash $0.05 in / $0.4 out per M at its lowest (≤256K) tier.
#[test]
fn qwen_flash_is_priced_at_the_published_rate() {
    assert_eq!(super::rate("qwen-flash"), Some((50_000, 400_000)));
    assert_eq!(
        super::rate("qwen-flash-2025-07-28"),
        Some((50_000, 400_000))
    );
}

/// Longest pattern wins, and a pattern is a substring, not a word: an alias
/// or a dated snapshot takes its family's row, and `qwen3-max` has its own
/// ($1.2/$6 at ≤32K) rather than `qwen-max`'s $1.6/$6.4.
#[test]
fn aliases_and_snapshots_find_their_own_family() {
    let qwen_max = Some((1_600_000, 6_400_000));
    assert_eq!(super::rate("qwen-max"), qwen_max);
    assert_eq!(super::rate("qwen-max-latest"), qwen_max);
    assert_eq!(super::rate("qwen3-max"), Some((1_200_000, 6_000_000)));
    assert_eq!(
        super::rate("qwen3-max-2026-01-23"),
        Some((1_200_000, 6_000_000))
    );
    assert_eq!(
        super::rate("qwen-plus-2025-12-01"),
        Some((400_000, 1_200_000))
    );
    assert_eq!(
        super::rate("qwen3-coder-plus"),
        Some((1_000_000, 5_000_000))
    );
    assert_eq!(super::rate("qwen3-coder-flash"), Some((300_000, 1_500_000)));
}

/// NVIDIA's free developer key and OpenRouter's `:free` slugs cost nothing —
/// LISTED at zero, so no caller falls back to a tier's Claude price for
/// them. A `:free` slug wins even when it contains a paid row's pattern.
#[test]
fn free_models_are_listed_at_zero() {
    assert_eq!(
        super::rate("nvidia/nemotron-3.5-lightning-30b-a3b"),
        Some((0, 0))
    );
    assert_eq!(
        super::rate("nvidia/nemotron-3-ultra-550b-a55b"),
        Some((0, 0))
    );
    assert_eq!(super::rate("openai/gpt-oss-20b:free"), Some((0, 0)));
    assert_eq!(super::rate("qwen/qwen3-coder:free"), Some((0, 0)));
}

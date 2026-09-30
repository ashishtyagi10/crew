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
    // $2/Mtok in + $10/Mtok out: 10k in + 1k out = 20_000 + 10_000 µ$.
    assert_eq!(
        cost_microusd("anthropic/Claude-Sonnet-5", 10_000, 1_000),
        30_000
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

/// Each provider chain's lead model at its vendor's own rate (2026-09-29),
/// none caught by a shorter row it contains: `gpt-5.5` is not `gpt-5`,
/// `qwen3.8-max` is not `qwen-max`, `deepseek-flash` is not `deepseek`.
#[test]
fn todays_default_models_bill_at_their_own_rates() {
    for (model, rate) in [
        ("qwen3.8-max", (2_000_000, 6_000_000)),
        ("qwen3.8-max-0902", (2_000_000, 6_000_000)),
        ("gpt-5.5", (5_000_000, 30_000_000)),
        ("gpt-5", (1_250_000, 10_000_000)),
        ("gemini-3.8-flash", (750_000, 3_750_000)),
        ("google/gemini-3.1-pro-preview", (2_000_000, 12_000_000)),
        ("deepseek-flash", (300_000, 1_200_000)),
    ] {
        assert_eq!(super::rate(model), Some(rate), "{model}");
    }
    // 100 × $2 + 900 × $0.25 + 10 × $6 per M: the page's cache read, not
    // the family's 20% of input ($0.40).
    assert_eq!(cached_reply_cost("qwen3.8-max"), 200 + 225 + 60);
}

/// 900 of a 1000-token prompt read from cache, 10 tokens out, at `model`.
fn cached_reply_cost(model: &str) -> u64 {
    let c = crate::provider::Completion {
        input_tokens: 1_000,
        cached_input_tokens: 900,
        output_tokens: 10,
        ..Default::default()
    };
    super::usage_cost(model, &c)
}

/// Anthropic's model table, 2026-09: the models that changed price have rows
/// longer than their family's, so the family's older rate cannot reach them.
/// Each cached rate is the table's own, not 0.1× input.
#[test]
fn current_claude_models_bill_at_their_own_rates() {
    assert_eq!(
        super::rate("claude-opus-5-5"),
        Some((4_000_000, 20_000_000))
    );
    // 100 × $4 + 900 × $0.20 + 10 × $20 per M.
    assert_eq!(cached_reply_cost("claude-opus-5-5"), 400 + 180 + 200);
    for sonnet in ["claude-sonnet-5", "claude-sonnet-5-5"] {
        assert_eq!(super::rate(sonnet), Some((2_000_000, 10_000_000)));
        // 100 × $2 + 900 × $0.20 + 10 × $10 per M.
        assert_eq!(cached_reply_cost(sonnet), 200 + 180 + 100, "{sonnet}");
    }
    assert_eq!(
        super::rate("claude-fable-5-1"),
        Some((10_000_000, 50_000_000))
    );
    // 100 × $10 + 900 × $0.25 + 10 × $50 per M.
    assert_eq!(cached_reply_cost("claude-fable-5-1"), 1_000 + 225 + 500);
}

/// OpenRouter spells the version with a dot; the same model costs the same.
#[test]
fn openrouter_aliases_bill_like_the_native_slug() {
    for (native, alias) in [
        ("claude-opus-5-5", "anthropic/claude-opus-5.5"),
        ("claude-sonnet-5-5", "anthropic/claude-sonnet-5.5"),
        ("claude-fable-5-1", "anthropic/claude-fable-5.1"),
    ] {
        assert_eq!(
            cached_reply_cost(alias),
            cached_reply_cost(native),
            "{alias}"
        );
    }
}

/// The family rows still price the models that kept their price.
#[test]
fn older_claude_models_keep_their_family_rate() {
    assert_eq!(
        super::rate("claude-opus-4-8"),
        Some((5_000_000, 25_000_000))
    );
    assert_eq!(super::rate("claude-opus-5"), Some((5_000_000, 25_000_000)));
    assert_eq!(
        super::rate("claude-sonnet-4-6"),
        Some((3_000_000, 15_000_000))
    );
    assert_eq!(
        super::rate("claude-fable-5"),
        Some((10_000_000, 50_000_000))
    );
    assert_eq!(cached_reply_cost("claude-fable-5"), 1_000 + 900 + 500);
}

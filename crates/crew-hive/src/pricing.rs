//! Approximate per-model API pricing, used to attach a dollar cost to token
//! usage when the provider doesn't report one exactly (OpenRouter does; see
//! `provider::openai_http`). Rates are micro-USD per 1M tokens, matched by
//! substring on the model slug (longest pattern first), so provider prefixes
//! (`anthropic/claude-sonnet-5`) and date suffixes both hit. Unknown models
//! cost 0 — the footer hides the `$` segment rather than invent a number.

/// (slug substring, input µ$/Mtok, output µ$/Mtok). Approximate list prices,
/// 2026-07. Order does not matter — the longest matching pattern wins.
const RATES: &[(&str, u64, u64)] = &[
    // Anthropic
    ("claude-opus", 15_000_000, 75_000_000),
    ("claude-sonnet", 3_000_000, 15_000_000),
    ("claude-haiku", 1_000_000, 5_000_000),
    ("claude-fable", 15_000_000, 75_000_000),
    // Qwen / DashScope
    ("qwen3-coder-plus", 1_000_000, 5_000_000),
    ("qwen3-coder-flash", 300_000, 1_500_000),
    ("qwen3-coder", 1_000_000, 5_000_000),
    ("qwen-max", 1_600_000, 6_400_000),
    ("qwen-plus", 400_000, 1_200_000),
    ("qwen-turbo", 50_000, 200_000),
    // OpenAI
    ("gpt-4o-mini", 150_000, 600_000),
    ("gpt-4o", 2_500_000, 10_000_000),
    ("gpt-4.1-mini", 400_000, 1_600_000),
    ("gpt-4.1", 2_000_000, 8_000_000),
    // DeepSeek
    ("deepseek-reasoner", 550_000, 2_190_000),
    ("deepseek", 270_000, 1_100_000),
    // Moonshot / Kimi
    ("kimi-k2", 600_000, 2_500_000),
];

/// Approximate cost of one reply in micro-USD; 0 when the model is unknown.
pub fn cost_microusd(model: &str, input_tokens: u32, output_tokens: u32) -> u64 {
    let m = model.to_ascii_lowercase();
    let Some((_, in_rate, out_rate)) = RATES
        .iter()
        .filter(|(pat, _, _)| m.contains(pat))
        .max_by_key(|(pat, _, _)| pat.len())
    else {
        return 0;
    };
    (in_rate * u64::from(input_tokens) + out_rate * u64::from(output_tokens)) / 1_000_000
}

#[cfg(test)]
mod tests {
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
}

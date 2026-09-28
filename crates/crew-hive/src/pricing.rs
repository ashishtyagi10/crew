//! Approximate per-model API pricing, used to attach a dollar cost to token
//! usage when the provider doesn't report one exactly (OpenRouter does; see
//! `provider::openai_http`). Rates are micro-USD per 1M tokens, matched by
//! substring on the model slug (longest pattern first), so provider prefixes
//! (`anthropic/claude-sonnet-5`) and date suffixes (`qwen-plus-2025-12-01`,
//! `qwen-max-latest`) both hit, and `qwen3-max` finds its own row rather
//! than `qwen-max`'s. Unknown models cost 0 — the footer hides the `$`
//! segment rather than invent a number.
use crate::provider::Completion;

/// (slug substring, input, output, cached input) in µ$/Mtok. Order does not
/// matter — the longest matching pattern wins. The cached column is what a
/// prompt-cache HIT costs (`Completion::cached_input_tokens`); where no
/// cached rate has been checked it repeats the input rate, so a cache hit is
/// never billed below what the provider published.
const RATES: &[(&str, u64, u64, u64)] = &[
    // Anthropic list prices, 2026-07. A cache read is 0.1× input; a cache
    // write is 1.25× ([`WRITE_NUM`]/[`WRITE_DEN`], the 5-minute cache).
    ("claude-opus", 5_000_000, 25_000_000, 500_000),
    ("claude-sonnet", 3_000_000, 15_000_000, 300_000),
    ("claude-haiku", 1_000_000, 5_000_000, 100_000),
    ("claude-fable", 10_000_000, 50_000_000, 1_000_000),
    // Qwen / DashScope, International (Singapore) deployment — Alibaba Cloud
    // Model Studio "Model pricing",
    // https://www.alibabacloud.com/help/en/model-studio/model-pricing, read
    // 2026-09-28 (page last updated that day). A model priced in tiers by
    // input length is at its LOWEST tier here (qwen-flash and qwen-plus
    // ≤256K, qwen3-max and qwen3-coder-* ≤32K): a longer prompt costs more
    // than this says. qwen-plus and qwen-turbo output is the non-thinking
    // price; their thinking-mode output is dearer ($4 and $0.5 per M).
    // Cached input is the implicit cache's 20% of input — "Context Cache",
    // https://www.alibabacloud.com/help/en/model-studio/context-cache, same
    // day — which every model below supports and crew cannot turn off.
    ("qwen-flash", 50_000, 400_000, 10_000),
    ("qwen-turbo", 50_000, 200_000, 10_000),
    ("qwen-plus", 400_000, 1_200_000, 80_000),
    ("qwen-max", 1_600_000, 6_400_000, 320_000),
    ("qwen3-max", 1_200_000, 6_000_000, 240_000),
    ("qwen3-coder-plus", 1_000_000, 5_000_000, 200_000),
    ("qwen3-coder-flash", 300_000, 1_500_000, 60_000),
    // Any other qwen3-coder slug, at the plus rate; its cache rate unknown.
    ("qwen3-coder", 1_000_000, 5_000_000, 1_000_000),
    // OpenAI, 2026-07 — cached rates not checked, so full input.
    ("gpt-4o-mini", 150_000, 600_000, 150_000),
    ("gpt-4o", 2_500_000, 10_000_000, 2_500_000),
    ("gpt-4.1-mini", 400_000, 1_600_000, 400_000),
    ("gpt-4.1", 2_000_000, 8_000_000, 2_000_000),
    // DeepSeek — cached rates not checked, so full input.
    ("deepseek-reasoner", 550_000, 2_190_000, 550_000),
    ("deepseek", 270_000, 1_100_000, 270_000),
    // Moonshot / Kimi — cached rate not checked, so full input.
    ("kimi-k2", 600_000, 2_500_000, 600_000),
    // NVIDIA NIM (build.nvidia.com): crew's free first-run key, and the free
    // developer tier has no per-token price. Listed at 0 rather than left
    // out, because an unlisted model is billed at its TIER's Claude model
    // (`apiagent::billed`) — a free call read as a Haiku one. OpenRouter's
    // paid `nvidia/…` slugs report their own cost, which outranks this row.
    ("nvidia/", 0, 0, 0),
];

/// What writing a token INTO a prompt cache costs, as a fraction of input:
/// 1.25× on both hosts that report writes (Anthropic's 5-minute cache,
/// DashScope's explicit cache).
const WRITE_NUM: u64 = 5;
const WRITE_DEN: u64 = 4;

/// `model`'s (input, output, cached input) rates, or `None` when unlisted.
/// An OpenRouter `:free` slug is free whatever it is a variant of — checked
/// first, since `qwen/qwen3-coder:free` also contains a paid row's pattern.
fn rates(model: &str) -> Option<(u64, u64, u64)> {
    let m = model.to_ascii_lowercase();
    if m.ends_with(":free") {
        return Some((0, 0, 0));
    }
    RATES
        .iter()
        .filter(|(pat, ..)| m.contains(pat))
        .max_by_key(|(pat, ..)| pat.len())
        .map(|&(_, i, o, c)| (i, o, c))
}

/// The (input, output) µ$/Mtok rates for `model`, or `None` when the list
/// has never heard of it — which is a different answer from "free", and the
/// one a caller with a fallback estimate needs.
pub fn rate(model: &str) -> Option<(u64, u64)> {
    rates(model).map(|(i, o, _)| (i, o))
}

/// Approximate cost of one reply in micro-USD; 0 when the model is unknown.
pub fn cost_microusd(model: &str, input_tokens: u32, output_tokens: u32) -> u64 {
    let Some((in_rate, out_rate)) = rate(model) else {
        return 0;
    };
    (in_rate * u64::from(input_tokens) + out_rate * u64::from(output_tokens)) / 1_000_000
}

/// Approximate cost of `c` at `model`'s rates, cache included: the prompt's
/// cache hits at the cached rate, its cache writes at 1.25× input, the rest
/// at the input rate. 0 when the model is unknown.
pub fn usage_cost(model: &str, c: &Completion) -> u64 {
    let Some((in_rate, out_rate, cached_rate)) = rates(model) else {
        return 0;
    };
    let read = u64::from(c.cached_input_tokens);
    let written = u64::from(c.cache_write_tokens);
    let fresh = u64::from(c.input_tokens).saturating_sub(read + written);
    (in_rate * fresh
        + cached_rate * read
        + in_rate * written * WRITE_NUM / WRITE_DEN
        + out_rate * u64::from(c.output_tokens))
        / 1_000_000
}

/// The listed model to price `c` at: the model that answered, else the one
/// `requested` — a response may spell its model in a way no row matches,
/// and the model asked for is then a nearer guess than none. `None` when
/// neither is listed.
pub fn priced_as<'a>(requested: &'a str, c: &'a Completion) -> Option<&'a str> {
    [c.model.as_str(), requested]
        .into_iter()
        .find(|m| rate(m).is_some())
}

/// [`usage_cost`] at [`priced_as`]'s model, or `None` when neither the
/// model that answered nor the one asked for is listed.
pub fn estimate(requested: &str, c: &Completion) -> Option<u64> {
    priced_as(requested, c).map(|m| usage_cost(m, c))
}

#[cfg(test)]
#[path = "pricing_tests.rs"]
mod tests;

//! Per-model context-window limits for the pulse lanes' ctx meter. Matched by
//! substring on the model slug (most specific entry first), so provider
//! prefixes and date-stamped variants ("qwen-max-2025-01-25",
//! "anthropic/claude-sonnet-5") still resolve. Conservative where a family
//! spans sizes; unknown models return `None` and the meter falls back to an
//! absolute token count.
const LIMITS: &[(&str, u64)] = &[
    // Alibaba DashScope / Qwen
    ("qwen-max", 32_768),
    ("qwen-plus", 131_072),
    ("qwen-turbo", 131_072),
    ("qwq", 131_072),
    ("qwen", 131_072), // qwen3-* family
    // Anthropic: every current model reads 1M (Anthropic's model table,
    // 2026-09) but Haiku 4.5, which falls through to the family's 200K with
    // every older model — a meter reading too full is the safe error. The
    // 4.x rows come in both spellings, OpenRouter's alias being dotted
    // (`anthropic/claude-opus-4.8`); a 5.x row already matches `-5.5`.
    ("claude-opus-4-6", 1_000_000),
    ("claude-opus-4.6", 1_000_000),
    ("claude-opus-4-7", 1_000_000),
    ("claude-opus-4.7", 1_000_000),
    ("claude-opus-4-8", 1_000_000),
    ("claude-opus-4.8", 1_000_000),
    ("claude-sonnet-4-6", 1_000_000),
    ("claude-sonnet-4.6", 1_000_000),
    ("claude-opus-5", 1_000_000),
    ("claude-sonnet-5", 1_000_000),
    ("claude-fable", 1_000_000),
    ("claude", 200_000),
    // OpenAI
    ("gpt-4o", 128_000),
    ("gpt-4.1", 1_000_000),
    ("gpt-5", 400_000),
    ("gpt", 128_000),
    ("o3", 200_000),
    ("o4", 200_000),
    // Google
    ("gemini", 1_000_000),
    // Open-weights families common on OpenRouter
    ("llama", 131_072),
    ("deepseek", 131_072),
    ("mistral", 131_072),
    ("mixtral", 32_768),
    ("kimi", 131_072),
    ("glm", 131_072),
];

/// The context-window size (tokens) for `model`, or `None` when unknown.
pub(crate) fn context_limit(model: &str) -> Option<u64> {
    let slug = model.to_ascii_lowercase();
    LIMITS
        .iter()
        .find(|(pat, _)| slug.contains(pat))
        .map(|(_, n)| *n)
}

#[cfg(test)]
#[path = "ctxlimit_tests.rs"]
mod tests;

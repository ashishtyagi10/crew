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
    // Anthropic
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
mod tests {
    use super::*;

    #[test]
    fn specific_entry_wins_over_family() {
        assert_eq!(context_limit("qwen-max"), Some(32_768));
        assert_eq!(context_limit("qwen-plus"), Some(131_072));
        assert_eq!(context_limit("qwen3-235b-a22b"), Some(131_072));
    }

    #[test]
    fn matches_through_prefixes_variants_and_case() {
        assert_eq!(context_limit("anthropic/claude-sonnet-5"), Some(200_000));
        assert_eq!(context_limit("qwen-max-2025-01-25"), Some(32_768));
        assert_eq!(context_limit("Qwen-Max"), Some(32_768));
        assert_eq!(
            context_limit("meta-llama/llama-3.3-70b-instruct:free"),
            Some(131_072)
        );
    }

    #[test]
    fn unknown_models_have_no_limit() {
        assert_eq!(context_limit("mystery-model-9000"), None);
        assert_eq!(context_limit(""), None);
    }
}

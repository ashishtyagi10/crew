use super::*;

#[test]
fn specific_entry_wins_over_family() {
    assert_eq!(context_limit("qwen-max"), Some(32_768));
    assert_eq!(context_limit("qwen-plus"), Some(131_072));
    assert_eq!(context_limit("qwen3-235b-a22b"), Some(131_072));
}

#[test]
fn matches_through_prefixes_variants_and_case() {
    assert_eq!(context_limit("anthropic/claude-sonnet-5"), Some(1_000_000));
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

/// Anthropic's current models read 1M, in either spelling of their version;
/// Haiku 4.5 and anything older stay at the family's conservative 200K.
#[test]
fn current_claude_models_have_a_million_haiku_and_older_do_not() {
    for m in [
        "claude-opus-5-5",
        "anthropic/claude-opus-5.5",
        "claude-sonnet-5-5",
        "claude-fable-5-1",
        "claude-opus-4-8",
        "anthropic/claude-opus-4.8",
        "claude-sonnet-4-6",
    ] {
        assert_eq!(context_limit(m), Some(1_000_000), "{m}");
    }
    for m in [
        "claude-haiku-4-5",
        "anthropic/claude-haiku-4.5",
        "claude-opus-4-5",
        "claude-3-5-sonnet",
    ] {
        assert_eq!(context_limit(m), Some(200_000), "{m}");
    }
}

/// The models the provider chains lead with today hold far more than their
/// family's row — qwen-max's 32K is what overflowed — and a meter that reads
/// them at the family size shows a session full long before it is.
#[test]
fn todays_default_models_read_their_own_window() {
    for (m, n) in [
        ("qwen3.8-max", 1_000_000),
        ("qwen3.8-max-0902", 1_000_000),
        ("qwen3.8-flash", 1_000_000),
        ("qwen3.7-max", 1_000_000),
        ("gpt-5.5", 1_050_000),
        ("openai/gpt-6-sol", 1_050_000),
        ("gpt-6.1-sol", 1_050_000),
        ("gpt-5", 400_000),
        ("gemini-3.8-flash", 1_000_000),
        ("deepseek-flash", 1_000_000),
        ("deepseek/deepseek-v4.1-flash", 1_000_000),
        ("deepseek-v4-pro", 1_000_000),
    ] {
        assert_eq!(context_limit(m), Some(n), "{m}");
    }
    assert_eq!(context_limit("qwen-max"), Some(32_768));
}

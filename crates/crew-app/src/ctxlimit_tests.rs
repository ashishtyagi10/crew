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

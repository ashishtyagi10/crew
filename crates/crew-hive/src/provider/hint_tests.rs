//! Each fix on the provider bodies that call for it, word for word as the
//! hosts send them, and the sentences that must NOT get one.
use crate::provider::{api_message, ProviderError};

/// The one hint an error got: what follows the em dash after the sentence.
fn hint_of(m: &str) -> &str {
    m.rsplit_once(" \u{2014} ").map_or("", |(_, h)| h)
}

#[test]
fn dashscope_input_past_the_context_says_start_a_fresh_pane() {
    let body = r#"{"error":{"message":"Range of input length should be [1, 30720]","type":"invalid_request_error","param":null,"code":"invalid_parameter_error"}}"#;
    let m = api_message(body);
    assert!(m.starts_with("api error: Range of input length"), "{m}");
    assert_eq!(
        hint_of(&m),
        "the conversation no longer fits the model's context; start a fresh pane"
    );
}

/// "40100" holds "401", which the key check used to find and blame the key.
#[test]
fn an_openai_context_overflow_is_not_read_as_a_bad_key() {
    let body = r#"{"error":{"message":"This model's maximum context length is 32768 tokens. However, you requested 40100 tokens (39000 in the messages, 1100 in the completion). Please reduce the length of the messages or completion.","type":"invalid_request_error","param":"messages","code":"context_length_exceeded"}}"#;
    let m = api_message(body);
    assert!(!m.contains("rejected the key"), "{m}");
    assert!(
        hint_of(&m).contains("no longer fits the model's context"),
        "{m}"
    );
}

#[test]
fn a_spent_quota_says_top_it_up() {
    let body = r#"{"error":{"message":"You exceeded your current quota, please check your plan and billing details. For more information on this error, read the docs: https://platform.openai.com/docs/guides/error-codes/api-errors.","type":"insufficient_quota","param":null,"code":"insufficient_quota"}}"#;
    let m = api_message(body);
    assert_eq!(
        hint_of(&m),
        "the account is out of credit; top it up, or /model picks another provider"
    );
    let anthropic = r#"{"type":"error","error":{"type":"invalid_request_error","message":"Your credit balance is too low to access the Anthropic API. Please go to Plans & Billing to upgrade or purchase credits."}}"#;
    assert!(hint_of(&api_message(anthropic)).contains("out of credit"));
}

#[test]
fn a_model_the_host_does_not_serve_is_named() {
    let body = r#"{"error":{"message":"The model `qwen-maxx` does not exist or you do not have access to it."}}"#;
    let m = api_message(body);
    assert_eq!(
        hint_of(&m),
        "this host does not serve qwen-maxx; /model picks another"
    );
    let anthropic =
        r#"{"type":"error","error":{"type":"not_found_error","message":"model: claude-sonnet-9"}}"#;
    assert_eq!(
        hint_of(&api_message(anthropic)),
        "this host does not serve claude-sonnet-9; /model picks another"
    );
}

/// OpenRouter's wrapped 429 says only "Provider returned error"; its code
/// is what says rate limit.
#[test]
fn a_rate_limit_that_outlasted_the_retries_says_wait() {
    let wrapped = r#"{"error":{"message":"Provider returned error","code":429,"metadata":{"raw":"qwen/qwen3-coder:free is temporarily rate-limited upstream."}}}"#;
    let busy = "rate limited; wait a minute, or pick another model with /model";
    assert_eq!(hint_of(&api_message(wrapped)), busy);
    let anthropic = r#"{"type":"error","error":{"type":"rate_limit_error","message":"This request would exceed the rate limit for your organization of 50,000 input tokens per minute."}}"#;
    assert_eq!(hint_of(&api_message(anthropic)), busy);
    let page = ProviderError::Status("HTTP 429 from api.example.com: Too Many Requests".into());
    assert_eq!(hint_of(&page.to_string()), busy);
}

/// First match wins: one error, one hint, and the key before anything else.
#[test]
fn an_error_gets_one_hint_and_the_key_comes_first() {
    let body = r#"{"error":{"message":"Incorrect API key provided: sk-abc***. You can find your API key at https://platform.openai.com/account/api-keys.","type":"invalid_request_error","code":"invalid_api_key"}}"#;
    let m = api_message(body);
    assert!(m.starts_with("provider rejected the key"), "{m}");
    assert_eq!(m.matches('\u{2014}').count(), 1, "{m}");
    let anthropic_key =
        r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
    assert!(api_message(anthropic_key).starts_with("provider rejected the key"));
}

/// Sentences near a row's words that are about something else.
#[test]
fn errors_a_person_cannot_fix_here_get_no_hint() {
    for body in [
        r#"{"error":{"message":"Unsupported parameter: 'max_tokens' is not supported with this model. Use 'max_completion_tokens' instead.","type":"invalid_request_error","param":"max_tokens","code":"unsupported_parameter"}}"#,
        r#"{"error":{"message":"This model is overloaded","type":"server_error"}}"#,
        r#"{"error":{"message":"Range of max_tokens should be [1, 8192]","type":"invalid_request_error","code":"invalid_parameter_error"}}"#,
    ] {
        let m = api_message(body);
        assert!(!m.contains('\u{2014}'), "{m}");
    }
    let page = ProviderError::Status("HTTP 404 from models.github.ai: Not Found".into());
    assert!(!page.to_string().contains('\u{2014}'), "{page}");
}

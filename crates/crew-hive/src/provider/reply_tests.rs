//! Reading each provider's reply, and refusing a key that is not there.
use super::*;
use crate::provider::AnthropicProvider;

#[test]
fn parse_response_extracts_text_and_usage() {
    let body = r#"{
        "content": [{"type": "text", "text": "Hello world"}],
        "usage": {"input_tokens": 12, "output_tokens": 5},
        "stop_reason": "end_turn"
    }"#;
    let c = AnthropicProvider::parse_response(body).unwrap();
    assert_eq!(c.text, "Hello world");
    assert_eq!(c.input_tokens, 12);
    assert_eq!(c.output_tokens, 5);
}

/// A `thinking` block is kept as the completion's thought — never as text.
#[test]
fn parse_response_keeps_a_thinking_block_as_thought_not_text() {
    let body = r#"{
        "content": [
            {"type": "thinking", "thinking": "let me see", "signature": "abc"},
            {"type": "text", "text": "Hello world"}
        ],
        "usage": {"input_tokens": 12, "output_tokens": 5}
    }"#;
    let c = AnthropicProvider::parse_response(body).unwrap();
    assert_eq!(c.text, "Hello world");
    assert_eq!(c.thought, "let me see");
}

#[test]
fn parse_response_errors_on_api_error_payload() {
    let body = r#"{"type":"error","error":{"type":"overloaded_error","message":"overloaded"}}"#;
    assert!(matches!(
        AnthropicProvider::parse_response(body),
        Err(ProviderError::Api(_))
    ));
}

/// The rule, tested against a value rather than the process environment.
///
/// This used to read `ANTHROPIC_API_KEY` and assert only when it was absent,
/// so on any machine that had one it asserted NOTHING and passed — silently,
/// and on exactly the machines most likely to run it. It also never covered
/// the empty-string case, which is the one that matters: an exported but
/// blank key is not a key.
#[test]
fn a_missing_or_blank_anthropic_key_errors() {
    assert!(matches!(
        AnthropicProvider::from_key(None),
        Err(ProviderError::MissingKey("ANTHROPIC_API_KEY"))
    ));
    assert!(matches!(
        AnthropicProvider::from_key(Some(String::new())),
        Err(ProviderError::MissingKey("ANTHROPIC_API_KEY"))
    ));
    assert!(AnthropicProvider::from_key(Some("sk-x".into())).is_ok());
}

#[test]
fn openrouter_parse_response_extracts_text_and_usage() {
    use super::openai_http::parse_response;
    // OpenAI-shaped response: choices[].message.content + usage token names.
    let body = r#"{
        "choices": [{"message": {"role": "assistant", "content": "Hello world"}}],
        "usage": {"prompt_tokens": 7, "completion_tokens": 3}
    }"#;
    let c = parse_response(body).unwrap();
    assert_eq!(c.text, "Hello world");
    assert_eq!(c.input_tokens, 7);
    assert_eq!(c.output_tokens, 3);
}

#[test]
fn openrouter_parse_response_errors_on_api_error_payload() {
    use super::openai_http::parse_response;
    let body = r#"{"error":{"code":402,"message":"insufficient credits"}}"#;
    assert!(matches!(parse_response(body), Err(ProviderError::Api(_))));
}

#[test]
fn parse_response_reads_openrouter_cost() {
    use super::openai_http::parse_response;
    let body = r#"{"choices":[{"message":{"content":"hi"}}],
        "usage":{"prompt_tokens":10,"completion_tokens":5,"cost":0.000129}}"#;
    let c = parse_response(body).unwrap();
    assert_eq!(c.cost_microusd, 129);
}

#[test]
fn parse_response_without_cost_is_zero() {
    use super::openai_http::parse_response;
    let body = r#"{"choices":[{"message":{"content":"hi"}}],
        "usage":{"prompt_tokens":10,"completion_tokens":5}}"#;
    assert_eq!(parse_response(body).unwrap().cost_microusd, 0);
}

#[test]
fn sse_parser_extracts_deltas_usage_and_done() {
    use super::sseframe::{parse_sse_frame, SseItem};
    let none: Vec<SseItem> = Vec::new();
    assert_eq!(parse_sse_frame(""), none);
    assert_eq!(parse_sse_frame(": keep-alive"), none);
    assert_eq!(parse_sse_frame("data: [DONE]"), vec![SseItem::Done]);
    assert_eq!(
        parse_sse_frame(r#"data: {"choices":[{"delta":{"content":"hel"}}]}"#),
        vec![SseItem::Delta("hel".into())]
    );
    // Role-only first frame: no content → nothing, not an error.
    assert_eq!(
        parse_sse_frame(r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#),
        none
    );
    // Usage frame (stream_options include_usage / final frame).
    assert_eq!(
        parse_sse_frame(
            r#"data: {"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":42}}"#,
        ),
        vec![SseItem::Usage(10, 42, 0)]
    );
    // OpenRouter usage frame with exact cost (dollars, converted to micro-USD).
    assert_eq!(
        parse_sse_frame(
            r#"data: {"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":42,"cost":0.000129}}"#,
        ),
        vec![SseItem::Usage(10, 42, 129)]
    );
    assert_eq!(parse_sse_frame("data: {not json"), none);
}

#[test]
fn a_missing_or_blank_openrouter_key_errors() {
    assert!(matches!(
        OpenRouterProvider::from_key(None),
        Err(ProviderError::MissingKey("OPENROUTER_API_KEY"))
    ));
    assert!(matches!(
        OpenRouterProvider::from_key(Some(String::new())),
        Err(ProviderError::MissingKey("OPENROUTER_API_KEY"))
    ));
    assert!(OpenRouterProvider::from_key(Some("sk-x".into())).is_ok());
}

/// The message names the variable that was actually missing. It used to say
/// `ANTHROPIC_API_KEY` whatever had failed — and `OpenRouterProvider` backs
/// six providers through six different variables, so it was usually wrong.
#[test]
fn a_missing_key_names_its_own_variable() {
    assert_eq!(
        ProviderError::MissingKey("OPENAI_API_KEY").to_string(),
        "OPENAI_API_KEY not set"
    );
    assert_eq!(
        ProviderError::MissingKey("ANTHROPIC_API_KEY").to_string(),
        "ANTHROPIC_API_KEY not set"
    );
}

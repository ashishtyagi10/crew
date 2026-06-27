use super::*;
use crate::provider::AnthropicProvider;

#[tokio::test]
async fn mock_provider_echoes_reply_and_counts() {
    let p = MockProvider {
        reply: "hello there".into(),
    };
    let c = p
        .complete(CompletionRequest {
            model: "m".into(),
            system: None,
            prompt: "one two three".into(),
            max_tokens: 100,
        })
        .await
        .unwrap();
    assert_eq!(c.text, "hello there");
    assert_eq!(c.input_tokens, 3);
    assert_eq!(c.output_tokens, 2);
}

#[test]
fn provider_is_object_safe() {
    let _p: Box<dyn Provider> = Box::new(MockProvider { reply: "x".into() });
}

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

#[test]
fn parse_response_errors_on_api_error_payload() {
    let body = r#"{"type":"error","error":{"type":"overloaded_error","message":"overloaded"}}"#;
    assert!(matches!(
        AnthropicProvider::parse_response(body),
        Err(ProviderError::Api(_))
    ));
}

#[test]
fn from_env_missing_key_errors() {
    // Only assert the error shape when the key is absent; skip otherwise.
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        assert!(matches!(
            AnthropicProvider::from_env(),
            Err(ProviderError::MissingKey)
        ));
    }
}

#[tokio::test]
#[ignore = "requires ANTHROPIC_API_KEY; run with --ignored"]
async fn live_anthropic_completion() {
    let p = AnthropicProvider::from_env().expect("key");
    let c = p
        .complete(CompletionRequest {
            model: "claude-haiku-4-5".into(),
            system: Some("Reply with exactly the word: pong".into()),
            prompt: "ping".into(),
            max_tokens: 16,
        })
        .await
        .unwrap();
    assert!(!c.text.is_empty());
    assert!(c.output_tokens > 0);
}

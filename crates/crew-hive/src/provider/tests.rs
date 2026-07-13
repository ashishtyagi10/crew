use super::*;
use crate::provider::AnthropicProvider;

fn test_request() -> CompletionRequest {
    CompletionRequest {
        model: "m".into(),
        system: None,
        prompt: "one two three".into(),
        max_tokens: 100,
    }
}

#[tokio::test]
async fn mock_streams_reply_in_chunks_then_completes() {
    let p = MockProvider {
        reply: "alpha beta gamma delta".to_string(),
    };
    let chunks = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let sink = chunks.clone();
    let on_chunk: ChunkFn = std::sync::Arc::new(move |s: &str| {
        sink.lock().unwrap().push(s.to_string());
    });
    let done = p
        .complete_streaming(test_request(), on_chunk)
        .await
        .unwrap();
    let got = chunks.lock().unwrap();
    assert!(
        got.len() >= 2,
        "reply arrives in at least 2 chunks: {got:?}"
    );
    assert_eq!(
        got.concat(),
        "alpha beta gamma delta",
        "chunks reassemble the reply"
    );
    assert_eq!(done.text, "alpha beta gamma delta");
}

#[tokio::test]
async fn default_streaming_falls_back_without_chunks() {
    // Any provider using the trait default must behave like complete().
    // MockProvider OVERRIDES it, so exercise the default through a tiny
    // local test provider that only implements `complete`.
    struct Plain;
    impl Provider for Plain {
        fn complete(
            &self,
            _req: CompletionRequest,
        ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
            Box::pin(async {
                Ok(Completion {
                    text: "whole".into(),
                    input_tokens: 1,
                    output_tokens: 1,
                })
            })
        }
    }
    let ticked = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let t = ticked.clone();
    let on_chunk: ChunkFn = std::sync::Arc::new(move |_| {
        t.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    });
    let done = Plain
        .complete_streaming(test_request(), on_chunk)
        .await
        .unwrap();
    assert_eq!(done.text, "whole");
    assert_eq!(
        ticked.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "default never chunks"
    );
}

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
fn sse_parser_extracts_deltas_usage_and_done() {
    use super::openai_http::{parse_sse_line, SseItem};
    assert!(matches!(parse_sse_line(""), SseItem::Skip));
    assert!(matches!(parse_sse_line(": keep-alive"), SseItem::Skip));
    assert!(matches!(parse_sse_line("data: [DONE]"), SseItem::Done));
    match parse_sse_line(r#"data: {"choices":[{"delta":{"content":"hel"}}]}"#) {
        SseItem::Delta(s) => assert_eq!(s, "hel"),
        _ => panic!("delta expected"),
    }
    // Role-only first frame: no content → Skip, not an error.
    assert!(matches!(
        parse_sse_line(r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#),
        SseItem::Skip
    ));
    // Usage frame (stream_options include_usage / final frame).
    match parse_sse_line(
        r#"data: {"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":42}}"#,
    ) {
        SseItem::Usage(i, o) => assert_eq!((i, o), (10, 42)),
        _ => panic!("usage expected"),
    }
    assert!(matches!(parse_sse_line("data: {not json"), SseItem::Skip));
}

#[test]
fn openrouter_from_env_missing_key_errors() {
    if std::env::var("OPENROUTER_API_KEY").is_err() {
        assert!(matches!(
            OpenRouterProvider::from_env(),
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

#[tokio::test]
async fn arc_dyn_provider_is_a_provider() {
    // The broker holds Arc<dyn Provider>; LlmPlanner<P: Provider> must accept it.
    fn takes_provider<P: crate::provider::Provider>(p: P) -> P {
        p
    }
    let arc: std::sync::Arc<dyn crate::provider::Provider> =
        std::sync::Arc::new(crate::provider::MockProvider { reply: "ok".into() });
    let p = takes_provider(arc);
    let got = p
        .complete(crate::provider::CompletionRequest {
            model: "mock".into(),
            system: None,
            prompt: "hi".into(),
            max_tokens: 16,
        })
        .await
        .unwrap();
    assert_eq!(got.text, "ok");
}

use super::*;
use crate::provider::AnthropicProvider;

pub(super) fn test_request() -> CompletionRequest {
    CompletionRequest {
        model: "m".into(),
        system: None,
        prompt: "one two three".into(),
        max_tokens: 100,
        ..Default::default()
    }
}

#[tokio::test]
async fn mock_streams_reply_in_chunks_then_completes() {
    let p = MockProvider {
        reply: "alpha beta gamma delta".to_string(),
    };
    let chunks = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let sink = chunks.clone();
    let on_chunk: ChunkFn = std::sync::Arc::new(move |c: Chunk<'_>| {
        if let Chunk::Text(s) = c {
            sink.lock().unwrap().push(s.to_string());
        }
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
                    cost_microusd: 0,
                    ..Default::default()
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
            ..Default::default()
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

/// The mock reasons when its reply says so, and the reasoning is not text.
#[tokio::test]
async fn mock_routes_a_think_fence_to_thought_before_the_text() {
    let p = MockProvider {
        reply: "<think>weigh it</think>alpha beta".to_string(),
    };
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(bool, String)>::new()));
    let sink = seen.clone();
    let on_chunk: ChunkFn = std::sync::Arc::new(move |c: Chunk<'_>| {
        sink.lock().unwrap().push(match c {
            Chunk::Text(s) => (false, s.to_string()),
            Chunk::Thought(s) => (true, s.to_string()),
        });
    });
    let done = p
        .complete_streaming(test_request(), on_chunk)
        .await
        .unwrap();
    let got = seen.lock().unwrap();
    assert_eq!(
        got[0],
        (true, "weigh it".to_string()),
        "thought first: {got:?}"
    );
    assert!(got[1..].iter().all(|(t, _)| !t), "then only text");
    assert_eq!(done.text, "alpha beta");
    assert_eq!(done.thought, "weigh it");
    assert_eq!(
        done.output_tokens, 2,
        "tokens count the reply, not the working"
    );
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
            ..Default::default()
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
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(got.text, "ok");
}

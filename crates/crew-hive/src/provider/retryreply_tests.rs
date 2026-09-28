//! The retry rule over a socket, on the OpenAI-compatible path: a reply is
//! asked for once whatever it talks about, and OpenRouter's wrapped 429 is
//! still asked again.
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::provider::anthropic::retry_tests::{serve, whole};
use crate::provider::{Chunk, ChunkFn, Completion, CompletionRequest, OpenRouterProvider};
use crate::provider::{Provider, ProviderError};

/// DashScope's compatible-mode reply, word for word but for the content: a
/// completion whose TEXT is about rate limiting.
const MENTIONS: &str = r#"{"choices":[{"message":{"role":"assistant","content":"add a rate limit to the endpoint"},"finish_reason":"stop","index":0,"logprobs":null}],"object":"chat.completion","usage":{"prompt_tokens":12,"completion_tokens":8,"total_tokens":20},"created":1735120033,"system_fingerprint":null,"model":"qwen-plus","id":"chatcmpl-7f5b"}"#;

/// OpenRouter's upstream rate limit, wrapped in a 200.
const WRAPPED_429: &str = r#"{"error":{"code":429,"message":"Rate limit exceeded"}}"#;

fn provider(base: &str) -> OpenRouterProvider {
    OpenRouterProvider::new("k".into()).with_endpoint(format!("{base}/v1/chat/completions"))
}

fn req() -> CompletionRequest {
    CompletionRequest {
        model: "qwen-plus".into(),
        prompt: "how do I protect this endpoint?".into(),
        max_tokens: 64,
        ..Default::default()
    }
}

async fn whole_call(base: &str) -> Result<Completion, ProviderError> {
    let fut = provider(base).complete(req());
    tokio::time::timeout(Duration::from_secs(10), fut)
        .await
        .expect("must not hang")
}

/// Streamed, from a host that ignores `"stream": true` and answers whole.
async fn stream_call(base: &str) -> Result<Completion, ProviderError> {
    let on_chunk: ChunkFn = Arc::new(|_: Chunk<'_>| {});
    let fut = provider(base).complete_streaming(req(), on_chunk);
    tokio::time::timeout(Duration::from_secs(10), fut)
        .await
        .expect("must not hang")
}

#[tokio::test]
async fn a_reply_that_mentions_a_rate_limit_is_asked_for_once() {
    let ok = || whole("200 OK", "", MENTIONS);
    let (base, asked) = serve(vec![ok(), ok(), ok()]);
    let c = whole_call(&base).await.unwrap();
    assert_eq!(c.text, "add a rate limit to the endpoint");
    assert_eq!(asked.load(Ordering::SeqCst), 1, "a reply is not an error");
}

#[tokio::test]
async fn a_streamed_reply_that_mentions_a_rate_limit_is_asked_for_once() {
    let ok = || whole("200 OK", "", MENTIONS);
    let (base, asked) = serve(vec![ok(), ok(), ok()]);
    let c = stream_call(&base).await.unwrap();
    assert_eq!(c.text, "add a rate limit to the endpoint");
    assert_eq!(asked.load(Ordering::SeqCst), 1, "a reply is not an error");
}

#[tokio::test]
async fn a_wrapped_429_is_still_asked_again() {
    let first = whole("200 OK", "", WRAPPED_429);
    let (base, asked) = serve(vec![first, whole("200 OK", "", MENTIONS)]);
    let c = whole_call(&base).await.unwrap();
    assert_eq!(c.text, "add a rate limit to the endpoint");
    assert_eq!(asked.load(Ordering::SeqCst), 2);
}

use std::time::Duration;

use super::{attempt_chain, OpenRouterProvider};
use crate::provider::openai_http::retry_delay;
use crate::provider::{CompletionRequest, Provider, ProviderError};

#[test]
fn chain_puts_requested_model_first_then_fallbacks() {
    let fb = vec!["b:free".to_string(), "c:free".to_string()];
    assert_eq!(
        attempt_chain("a:free", &fb),
        vec!["a:free", "b:free", "c:free"]
    );
}

#[test]
fn chain_dedups_and_skips_empty() {
    // The requested model also appearing in the fallbacks isn't tried twice,
    // and empty entries are dropped.
    let fb = vec!["a:free".to_string(), String::new(), "b:free".to_string()];
    assert_eq!(attempt_chain("a:free", &fb), vec!["a:free", "b:free"]);
}

#[test]
fn chain_of_one_when_no_fallbacks() {
    assert_eq!(attempt_chain("only:free", &[]), vec!["only:free"]);
}

// The exact OpenRouter free-tier body the user hit: a 200 wrapping an
// upstream 429 with a Retry-After hint in metadata.
const RL_BODY: &str = r#"{"error":{"message":"Provider returned error","code":429,"metadata":{"raw":"... temporarily rate-limited upstream ...","retry_after_seconds":3.719}}}"#;

#[test]
fn retries_wrapped_429_using_body_hint() {
    // 200 status, but the body carries code 429 → retry, ceil(3.719)=4s.
    assert_eq!(retry_delay(200, None, RL_BODY, 0), Some(4));
}

#[test]
fn retry_after_header_wins_and_clamps() {
    // Header present → used, then clamped into [1, 8].
    assert_eq!(retry_delay(429, Some(2), "{}", 0), Some(2));
    assert_eq!(retry_delay(429, Some(999), "{}", 0), Some(8));
}

#[test]
fn exponential_backoff_when_no_hint() {
    assert_eq!(retry_delay(503, None, "", 0), Some(1));
    assert_eq!(retry_delay(503, None, "", 2), Some(4));
}

// A network stall (dead pooled socket, silent packet drop to the endpoint)
// looks like a server that accepts the connection and then never says
// anything. The provider must fail on its own — with a client-level timeout —
// rather than hang until the broker's outer per-call cap (180s) kills the
// whole attempt, which masks the cause and never reaches the fallback chain.
#[tokio::test]
async fn stalled_server_fails_fast_instead_of_hanging() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((sock, _)) = listener.accept().await {
            held.push(sock); // keep the connection open, never respond
        }
    });
    let p = OpenRouterProvider::new("k".into())
        .with_endpoint(format!("http://{addr}/v1/chat/completions"))
        .with_timeout(Duration::from_millis(200));
    let req = CompletionRequest {
        model: "m".into(),
        system: None,
        prompt: "hi".into(),
        max_tokens: 8,
    };
    let res = tokio::time::timeout(Duration::from_secs(5), p.complete(req))
        .await
        .expect("provider hung: it must time out on its own, not rely on an outer cap");
    assert!(
        matches!(res, Err(ProviderError::Http(_))),
        "expected an http timeout error, got {res:?}"
    );
}

#[test]
fn does_not_retry_hard_errors() {
    assert_eq!(
        retry_delay(400, None, r#"{"error":"bad request"}"#, 0),
        None
    );
    assert_eq!(retry_delay(401, None, "unauthorized", 0), None);
    assert_eq!(retry_delay(200, None, r#"{"choices":[]}"#, 0), None);
}

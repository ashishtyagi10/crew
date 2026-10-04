//! A request that never got an answer — the connection closed before any
//! status came back — is asked again, on both wire paths, and stands only
//! once the budget is spent.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::again_unanswered;
use crate::provider::{AnthropicProvider, CompletionRequest, OpenRouterProvider, Provider};

/// A loopback server that drops the first `drop` connections unanswered,
/// then answers with `ok` (a whole JSON 200). Returns its URL and how many
/// connections arrived.
fn flaky(drop: usize, ok: &'static str) -> (String, Arc<AtomicUsize>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
    let seen = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&seen);
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            let n = count.fetch_add(1, Ordering::SeqCst);
            let mut buf = [0u8; 8192];
            let _ = sock.read(&mut buf).await;
            if n >= drop {
                let reply = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{ok}",
                    ok.len()
                );
                let _ = sock.write_all(reply.as_bytes()).await;
            }
            let _ = sock.shutdown().await; // dropped: closed with no answer at all
        }
    });
    (format!("http://{addr}"), seen)
}

fn req(model: &str) -> CompletionRequest {
    CompletionRequest {
        model: model.into(),
        prompt: "hi".into(),
        max_tokens: 16,
        ..Default::default()
    }
}

const ANTHROPIC_OK: &str = r#"{"type":"message","content":[{"type":"text","text":"ok"}],"usage":{"input_tokens":1,"output_tokens":1}}"#;
const OPENAI_OK: &str = r#"{"choices":[{"message":{"content":"ok"}}],"usage":{"prompt_tokens":1,"completion_tokens":1}}"#;

#[test]
fn the_budget_is_two_tries_one_then_two_seconds_apart() {
    let mut attempt = 0;
    assert_eq!(again_unanswered(&mut attempt), Some(Duration::from_secs(1)));
    assert_eq!(again_unanswered(&mut attempt), Some(Duration::from_secs(2)));
    assert_eq!(again_unanswered(&mut attempt), None);
}

#[tokio::test]
async fn an_unanswered_anthropic_request_is_asked_again() {
    let (base, seen) = flaky(1, ANTHROPIC_OK);
    let p = AnthropicProvider::new("k".into()).with_base_url(&base);
    let got = tokio::time::timeout(Duration::from_secs(10), p.complete(req("claude-x"))).await;
    assert_eq!(got.expect("must not hang").unwrap().text, "ok");
    assert_eq!(seen.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn an_unanswered_openai_compatible_request_is_asked_again() {
    let (base, seen) = flaky(1, OPENAI_OK);
    let p =
        OpenRouterProvider::new("k".into()).with_endpoint(format!("{base}/v1/chat/completions"));
    let got = tokio::time::timeout(Duration::from_secs(10), p.complete(req("m"))).await;
    assert_eq!(got.expect("must not hang").unwrap().text, "ok");
    assert_eq!(seen.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_host_that_never_answers_fails_after_the_budget() {
    let (base, seen) = flaky(usize::MAX, ANTHROPIC_OK);
    let p = AnthropicProvider::new("k".into()).with_base_url(&base);
    let got = tokio::time::timeout(Duration::from_secs(10), p.complete(req("claude-x"))).await;
    assert!(got.expect("must not hang").is_err());
    assert_eq!(seen.load(Ordering::SeqCst), 3, "the first try and two more");
}

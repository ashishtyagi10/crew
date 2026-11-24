//! The Anthropic chain over a socket: a model that is busy or gone hands the
//! request to the next one, which says so; a refused key or account stops at
//! once; each rung's body is built for that rung; shown text is never replayed.
use super::*;
use crate::provider::anthropic::retry_tests::{delta, error_event, events, read_request, whole};
use crate::provider::Effort;
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

pub(super) const OVERLOADED: &str =
    r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#;
pub(super) const GONE: &str =
    r#"{"type":"error","error":{"type":"not_found_error","message":"model: claude-opus-5-5"}}"#;
pub(super) const KEY: &str =
    r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
pub(super) const CREDIT: &str = r#"{"type":"error","error":{"type":"invalid_request_error","message":"Your credit balance is too low to access the Anthropic API. Please go to Plans & Billing to upgrade or purchase credits."}}"#;
const OK: &str = r#"{"type":"message","content":[{"type":"text","text":"ok"}],"usage":{"input_tokens":1,"output_tokens":1}}"#;

/// A loopback server answering the Nth request with `replies[N]`, keeping
/// each request's JSON body so a test can read what each rung asked.
fn serve(replies: Vec<Vec<Vec<u8>>>) -> (String, Arc<Mutex<Vec<serde_json::Value>>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
    let bodies = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&bodies);
    tokio::spawn(async move {
        for reply in replies {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let req = read_request(&mut sock).await;
            let at = req.windows(4).position(|w| w == b"\r\n\r\n").unwrap_or(0);
            let body = serde_json::from_slice(&req[at + 4..]).unwrap_or_default();
            kept.lock().unwrap().push(body);
            for part in reply {
                let _ = sock.write_all(&part).await;
                let _ = sock.flush().await;
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            let _ = sock.shutdown().await;
        }
    });
    (format!("http://{addr}"), bodies)
}

fn busy() -> Vec<Vec<u8>> {
    whole("529 Overloaded", "retry-after: 0\r\n", OVERLOADED)
}

fn req(model: &str) -> CompletionRequest {
    CompletionRequest {
        model: model.into(),
        prompt: "hi".into(),
        max_tokens: 16,
        ..Default::default()
    }
}

fn chain(base: &str, configured: &[&str]) -> AnthropicChain {
    let configured = configured.iter().map(|m| m.to_string()).collect();
    AnthropicProvider::new("k".into())
        .with_base_url(base)
        .with_fallbacks(configured)
}

async fn ask(p: &AnthropicChain, r: CompletionRequest) -> Result<Completion, ProviderError> {
    let res = tokio::time::timeout(Duration::from_secs(20), p.complete(r)).await;
    res.expect("must not hang")
}

fn models(bodies: &Mutex<Vec<serde_json::Value>>) -> Vec<String> {
    let b = bodies.lock().unwrap();
    b.iter()
        .map(|v| v["model"].as_str().unwrap_or("").to_string())
        .collect()
}

#[tokio::test]
async fn an_overloaded_model_after_its_retries_hands_on_and_says_so() {
    let ok = whole("200 OK", "", OK);
    let (base, bodies) = serve(vec![busy(), busy(), busy(), ok]);
    let c = ask(&chain(&base, &[]), req("claude-opus-5-5"))
        .await
        .unwrap();
    assert_eq!(c.text, "ok");
    assert_eq!(
        c.model, "claude-sonnet-5-5",
        "billed at the model that answered"
    );
    assert!(
        c.fell_back.starts_with("claude-opus-5-5: Overloaded"),
        "{}",
        c.fell_back
    );
    let asked = models(&bodies);
    assert_eq!(
        asked,
        ["claude-opus-5-5"; 3]
            .into_iter()
            .chain(["claude-sonnet-5-5"])
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn a_retired_model_404_walks_on() {
    let gone = whole("404 Not Found", "", GONE);
    let (base, bodies) = serve(vec![gone, whole("200 OK", "", OK)]);
    let c = ask(&chain(&base, &[]), req("claude-sonnet-5-5"))
        .await
        .unwrap();
    assert_eq!(c.model, "claude-haiku-5-5");
    assert!(
        c.fell_back.contains("model: claude-opus-5-5"),
        "{}",
        c.fell_back
    );
    assert_eq!(models(&bodies), ["claude-sonnet-5-5", "claude-haiku-5-5"]);
}

#[tokio::test]
async fn a_refused_key_or_an_empty_account_stops_at_once() {
    for (status, body) in [("401 Unauthorized", KEY), ("400 Bad Request", CREDIT)] {
        let (base, bodies) = serve(vec![whole(status, "", body), whole("200 OK", "", OK)]);
        let res = ask(&chain(&base, &[]), req("claude-opus-5-5")).await;
        assert!(res.is_err(), "{status}");
        assert_eq!(
            bodies.lock().unwrap().len(),
            1,
            "{status}: no second rung asked"
        );
    }
}

#[tokio::test]
async fn each_rung_is_asked_with_a_body_built_for_it() {
    // Opus 5.5 takes `effort`; Sonnet 4.5 refuses it, so its body must not carry it.
    let gone = whole("404 Not Found", "", GONE);
    let (base, bodies) = serve(vec![gone, whole("200 OK", "", OK)]);
    let p = chain(&base, &["claude-opus-5-5", "claude-sonnet-4-5"]);
    let r = CompletionRequest {
        effort: Some(Effort::High),
        ..req("claude-opus-5-5")
    };
    assert_eq!(ask(&p, r).await.unwrap().model, "claude-sonnet-4-5");
    let b = bodies.lock().unwrap();
    assert_eq!(b[0]["output_config"]["effort"], "high");
    assert_eq!(b[1]["model"], "claude-sonnet-4-5");
    assert!(b[1].get("output_config").is_none(), "{}", b[1]);
    for key in ["temperature", "top_p", "tool_choice"] {
        assert!(b[1].get(key).is_none(), "{key} in {}", b[1]);
    }
}

#[tokio::test]
async fn a_stream_that_has_typed_a_word_is_not_handed_on() {
    let first = events(&[delta("par").as_bytes(), error_event().as_bytes()]);
    let (base, bodies) = serve(vec![first, events(&[delta("ok").as_bytes()])]);
    let shown = Arc::new(Mutex::new(String::new()));
    let s = Arc::clone(&shown);
    let on_chunk: ChunkFn = Arc::new(move |c| {
        if let crate::provider::Chunk::Text(t) = c {
            s.lock().unwrap().push_str(t);
        }
    });
    let fut = chain(&base, &[]).complete_streaming(req("claude-opus-5-5"), on_chunk);
    let res = tokio::time::timeout(Duration::from_secs(20), fut)
        .await
        .unwrap();
    assert!(res.is_err());
    assert_eq!(*shown.lock().unwrap(), "par");
    assert_eq!(bodies.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_stream_refused_before_a_word_hands_on() {
    let gone = whole("404 Not Found", "", GONE);
    let (base, bodies) = serve(vec![gone, events(&[delta("ok").as_bytes()])]);
    let fut = chain(&base, &[]).complete_streaming(req("claude-opus-5-5"), Arc::new(|_| {}));
    let c = tokio::time::timeout(Duration::from_secs(20), fut)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(c.text, "ok");
    assert!(!c.fell_back.is_empty());
    assert_eq!(models(&bodies), ["claude-opus-5-5", "claude-sonnet-5-5"]);
}

//! A failed answer keeps its status and host: the sentence alone, then a
//! proxy's HTML page over a socket on both HTTP paths.
use super::*;
use std::sync::Arc;
use std::time::Duration;

use crate::provider::anthropic::retry_tests::{serve, Reply};
use crate::provider::{AnthropicProvider, Chunk, ChunkFn, CompletionRequest};
use crate::provider::{OpenRouterProvider, Provider};

/// The page Tengine (the proxy in front of DashScope) answers a 502 with.
const TENGINE_502: &str = "<!DOCTYPE HTML PUBLIC \"-//IETF//DTD HTML 2.0//EN\">\r\n<html>\r\n<head><title>502 Bad Gateway</title></head>\r\n<body>\r\n<center><h1>502 Bad Gateway</h1></center>\r\n<hr><center>tengine</center>\r\n</body>\r\n</html>\r\n";

const HOST: &str = "dashscope-intl.aliyuncs.com";
const ENDPOINT: &str = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions";

#[test]
fn a_proxy_page_is_its_status_its_reason_and_its_host() {
    assert_eq!(
        sentence(502, HOST, TENGINE_502),
        "HTTP 502 from dashscope-intl.aliyuncs.com: Bad Gateway"
    );
    let Err(e) = settle(502, ENDPOINT, TENGINE_502, |_| unreachable!()) else {
        panic!("a 502 is not a reply");
    };
    assert_eq!(
        e.to_string(),
        "HTTP 502 from dashscope-intl.aliyuncs.com: Bad Gateway"
    );
}

#[test]
fn a_title_that_adds_something_is_kept_on_one_bounded_line() {
    let page = "<html><head><title>\n  api.example.com | 503:\n Origin is  unreachable </title></head></html>";
    let s = sentence(503, "api.example.com", page);
    assert_eq!(
        s,
        "HTTP 503 from api.example.com: Service Unavailable \u{2014} api.example.com | 503: Origin is unreachable"
    );
    let long = format!("<title>{}</title>", "gateway trouble ".repeat(20));
    let s = sentence(504, "h", &long);
    assert_eq!(s.chars().count(), MAX, "{s}");
    assert!(s.ends_with('\u{2026}') && !s.contains('\n'), "{s}");
}

/// No title: the page's text, tags gone. A status with no reason phrase
/// (Anthropic's 529) still says its number.
#[test]
fn without_a_title_the_text_speaks_and_every_status_has_a_number() {
    let s = sentence(
        503,
        "h",
        "<p>upstream connect error or disconnect/reset</p>",
    );
    assert_eq!(
        s,
        "HTTP 503 from h: Service Unavailable \u{2014} upstream connect error or disconnect/reset"
    );
    assert_eq!(sentence(529, "h", ""), "HTTP 529 from h");
    assert_eq!(
        sentence(200, "h", ""),
        "HTTP 200 from h: the reply is empty"
    );
    let portal = "<html><title>Sign in to Wi-Fi</title></html>";
    assert_eq!(
        sentence(200, "h", portal),
        "HTTP 200 from h: the reply is not JSON \u{2014} Sign in to Wi-Fi"
    );
}

/// A JSON envelope is the provider talking; its sentence stays the error.
#[test]
fn a_json_error_envelope_keeps_the_providers_sentence() {
    let body = r#"{"error":{"message":"Range of input length should be [1, 30720]","type":"invalid_request_error"}}"#;
    assert!(matches!(failure(400, ENDPOINT, body), ProviderError::Api(ref b) if b == body));
    let bare = r#"{"detail":"Not Found"}"#;
    let e = failure(404, ENDPOINT, bare).to_string();
    assert!(
        e.starts_with("HTTP 404 from dashscope-intl.aliyuncs.com: Not Found"),
        "{e}"
    );
}

/// Three 502 pages (the first try and both retries), each saying "now" so the
/// test does not sit through the backoff.
fn three_502s() -> Vec<Reply> {
    let head = format!(
        "HTTP/1.1 502 Bad Gateway\r\ncontent-type: text/html\r\nretry-after: 0\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        TENGINE_502.len()
    );
    let one = || vec![[head.as_bytes(), TENGINE_502.as_bytes()].concat()];
    vec![one(), one(), one()]
}

fn req() -> CompletionRequest {
    CompletionRequest {
        model: "qwen-plus".into(),
        prompt: "hi".into(),
        max_tokens: 16,
        ..Default::default()
    }
}

/// What the error card would read, for a call that must fail.
async fn said(fut: impl std::future::Future<Output = Result<Completion, ProviderError>>) -> String {
    let res = tokio::time::timeout(Duration::from_secs(10), fut).await;
    res.expect("must not hang").unwrap_err().to_string()
}

fn assert_502(e: &str, base: &str) {
    let host = base.trim_start_matches("http://");
    assert_eq!(e, format!("HTTP 502 from {host}: Bad Gateway"));
    assert!(!e.contains("decode error"), "{e}");
}

#[tokio::test]
async fn a_502_page_on_the_openai_path_reads_as_a_502() {
    let (base, _) = serve(three_502s());
    let p =
        OpenRouterProvider::new("k".into()).with_endpoint(format!("{base}/v1/chat/completions"));
    assert_502(&said(p.complete(req())).await, &base);
    let (base, _) = serve(three_502s());
    let p =
        OpenRouterProvider::new("k".into()).with_endpoint(format!("{base}/v1/chat/completions"));
    let on_chunk: ChunkFn = Arc::new(|_: Chunk<'_>| {});
    assert_502(&said(p.complete_streaming(req(), on_chunk)).await, &base);
}

#[tokio::test]
async fn a_502_page_on_the_anthropic_path_reads_as_a_502() {
    let (base, asked) = serve(three_502s());
    let p = AnthropicProvider::new("k".into()).with_base_url(&base);
    assert_502(&said(p.complete(req())).await, &base);
    assert_eq!(asked.load(std::sync::atomic::Ordering::SeqCst), 3);
}

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{attempt_chain, OpenRouterProvider};
use crate::provider::openai_http::retry_delay;
use crate::provider::{ChunkFn, CompletionRequest, Provider, ProviderError};

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

// --- complete_streaming over a real HTTP connection ------------------------
//
// A raw-TCP one-shot server: full control over the exact bytes sent back,
// no mock-HTTP-crate dependency needed (mirrors `stalled_server_fails_fast...`
// above, which already talks to a bare `TcpListener`). Accepts up to
// `max_conns` connections; each gets `bytes` written back, then the socket
// closes. `accepted` lets a test assert exactly how many connection attempts
// were made — e.g. that a fallback model was never dialed.
fn one_shot_server(bytes: Vec<u8>, max_conns: usize) -> (std::net::SocketAddr, Arc<AtomicUsize>) {
    use tokio::io::AsyncWriteExt;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let counted = accepted.clone();
    tokio::spawn(async move {
        for _ in 0..max_conns {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            counted.fetch_add(1, Ordering::SeqCst);
            let _ = sock.write_all(&bytes).await;
            let _ = sock.shutdown().await;
        }
    });
    (addr, accepted)
}

fn collecting_chunk_fn() -> (ChunkFn, Arc<Mutex<Vec<String>>>) {
    let chunks = Arc::new(Mutex::new(Vec::<String>::new()));
    let sink = chunks.clone();
    let on_chunk: ChunkFn = Arc::new(move |s: &str| sink.lock().unwrap().push(s.to_string()));
    (on_chunk, chunks)
}

/// Read one full HTTP/1.1 request (headers + `Content-Length` body) off a
/// freshly accepted socket, for a test server that needs to inspect what the
/// client actually sent (e.g. whether a retry dropped `stream_options`).
async fn read_request(sock: &mut tokio::net::TcpStream) -> Vec<u8> {
    use tokio::io::AsyncReadExt;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let n = sock.read(&mut chunk).await.unwrap_or(0);
        if n == 0 {
            return buf;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
            break pos + 4;
        }
    };
    let headers = String::from_utf8_lossy(&buf[..header_end]).to_lowercase();
    let content_len: usize = headers
        .split("content-length:")
        .nth(1)
        .and_then(|s| s.split("\r\n").next())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    while buf.len() < header_end + content_len {
        let n = sock.read(&mut chunk).await.unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    buf
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[tokio::test]
async fn streams_deltas_and_reports_final_usage_over_http() {
    let sse_body = concat!(
        "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n",
        "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":2}}\n",
        "data: [DONE]\n",
    );
    let head = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\n\r\n",
        sse_body.len()
    );
    let (addr, _accepted) = one_shot_server([head.as_bytes(), sse_body.as_bytes()].concat(), 1);
    let p = OpenRouterProvider::new("k".into())
        .with_endpoint(format!("http://{addr}/v1/chat/completions"));
    let (on_chunk, chunks) = collecting_chunk_fn();
    let req = CompletionRequest {
        model: "m".into(),
        system: None,
        prompt: "hi".into(),
        max_tokens: 8,
    };
    let done = p.complete_streaming(req, on_chunk).await.unwrap();
    assert_eq!(chunks.lock().unwrap().concat(), "Hello");
    assert_eq!(done.text, "Hello");
    assert_eq!(done.input_tokens, 5);
    assert_eq!(done.output_tokens, 2);
}

#[tokio::test]
async fn midstream_transport_error_does_not_fall_back_to_another_model() {
    // Chunked encoding: one complete, real delta chunk, then the socket
    // closes without the terminating zero-length chunk — an unexpected EOF
    // partway through the body, i.e. a genuine transport error, not a clean
    // end of stream.
    let delta = "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n";
    let framed_chunk = format!("{:x}\r\n{}\r\n", delta.len(), delta);
    let head =
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n";
    let (addr, accepted) = one_shot_server(
        [head.as_bytes(), framed_chunk.as_bytes()].concat(),
        // A correct implementation dials this server exactly once; allow up
        // to 2 accepts so a regression that DOES fall back to the next model
        // is observed as a second accepted connection rather than a hang.
        2,
    );
    let p = OpenRouterProvider::new("k".into())
        .with_endpoint(format!("http://{addr}/v1/chat/completions"))
        .with_fallbacks(vec!["backup:free".into()]);
    let (on_chunk, chunks) = collecting_chunk_fn();
    let req = CompletionRequest {
        model: "primary:free".into(),
        system: None,
        prompt: "hi".into(),
        max_tokens: 8,
    };
    let res = tokio::time::timeout(Duration::from_secs(5), p.complete_streaming(req, on_chunk))
        .await
        .expect("must not hang");
    assert!(
        matches!(res, Err(ProviderError::Http(_))),
        "expected a transport error, got {res:?}"
    );
    assert_eq!(
        chunks.lock().unwrap().as_slice(),
        &["partial".to_string()],
        "the delta seen before the failure was still forwarded"
    );
    // Give a buggy fallback attempt a moment to have dialed in, then assert
    // it never did.
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        accepted.load(Ordering::SeqCst),
        1,
        "must not have retried against another model after streaming visible content"
    );
}

#[tokio::test]
async fn retries_once_without_stream_options_on_400() {
    use tokio::io::AsyncWriteExt;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
    let saw_stream_options = Arc::new(Mutex::new(Vec::<bool>::new()));
    let seen = saw_stream_options.clone();
    tokio::spawn(async move {
        for i in 0..2 {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let req = read_request(&mut sock).await;
            seen.lock()
                .unwrap()
                .push(String::from_utf8_lossy(&req).contains("stream_options"));
            let (status_line, body) = if i == 0 {
                (
                    "HTTP/1.1 400 Bad Request",
                    r#"{"error":{"message":"stream_options not supported"}}"#,
                )
            } else {
                (
                    "HTTP/1.1 200 OK",
                    "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\ndata: [DONE]\n",
                )
            };
            let head = format!(
                "{status_line}\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\n\r\n",
                body.len()
            );
            let _ = sock.write_all(head.as_bytes()).await;
            let _ = sock.write_all(body.as_bytes()).await;
            let _ = sock.shutdown().await;
        }
    });
    let p = OpenRouterProvider::new("k".into())
        .with_endpoint(format!("http://{addr}/v1/chat/completions"));
    let (on_chunk, chunks) = collecting_chunk_fn();
    let req = CompletionRequest {
        model: "m".into(),
        system: None,
        prompt: "hi".into(),
        max_tokens: 8,
    };
    let done = tokio::time::timeout(Duration::from_secs(5), p.complete_streaming(req, on_chunk))
        .await
        .expect("must not hang")
        .unwrap();
    assert_eq!(done.text, "ok");
    assert_eq!(chunks.lock().unwrap().concat(), "ok");
    let seen = saw_stream_options.lock().unwrap();
    assert_eq!(seen.len(), 2, "expected exactly two attempts: {seen:?}");
    assert!(seen[0], "first attempt must include stream_options");
    assert!(!seen[1], "retry after the 400 must drop stream_options");
}

// --- Critical-1: 200-with-wrapped-error-body must not become a silent
// empty success ------------------------------------------------------------
//
// The exact shape from `retry_delay`'s doc comment: OpenRouter returns
// upstream rate-limits as a 200 whose body is a plain JSON `error` object —
// no `data:` lines at all. Treated naively as an SSE stream, every line
// would Skip and the call would return `Ok(Completion{text: "", ..})` with
// fabricated usage, defeating both retry and the fallback chain. No
// `retry_after_seconds` hint here, so `retry_delay` backs off
// exponentially (1s, then 2s) rather than the 4s the doc-comment example's
// hint would incur — keeps this test fast.
const WRAPPED_ERROR_BODY: &str = r#"{"error":{"message":"Provider returned error","code":429}}"#;

#[tokio::test]
async fn wrapped_error_200_body_with_json_content_type_retries_then_errors() {
    // content-type: application/json, no `data:` framing — exercises the
    // belt-and-braces content-type gate straight to the non-streaming error
    // handling, without ever attempting to parse it as a stream.
    let head = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",
        WRAPPED_ERROR_BODY.len()
    );
    let (addr, accepted) = one_shot_server(
        [head.as_bytes(), WRAPPED_ERROR_BODY.as_bytes()].concat(),
        3, // initial attempt + 2 retries (MAX_RETRIES = 2)
    );
    let p = OpenRouterProvider::new("k".into())
        .with_endpoint(format!("http://{addr}/v1/chat/completions"));
    let (on_chunk, chunks) = collecting_chunk_fn();
    let req = CompletionRequest {
        model: "m".into(),
        system: None,
        prompt: "hi".into(),
        max_tokens: 8,
    };
    let res = tokio::time::timeout(Duration::from_secs(10), p.complete_streaming(req, on_chunk))
        .await
        .expect("must not hang");
    assert!(
        matches!(res, Err(ProviderError::Api(_))),
        "wrapped 200 error body must surface as an error, not a fabricated empty success; got {res:?}"
    );
    assert!(
        chunks.lock().unwrap().is_empty(),
        "no delta was ever produced"
    );
    assert_eq!(
        accepted.load(Ordering::SeqCst),
        3,
        "must have retried through MAX_RETRIES before giving up"
    );
}

#[tokio::test]
async fn wrapped_error_200_body_without_content_type_is_not_silent_success() {
    // No content-type header at all — falls through to consume_sse, which
    // must recognize zero Done/Delta/Usage frames ever arrived and hand the
    // raw body to the same parse_response/retry_delay handling as the
    // non-streaming path, rather than returning an empty successful stream.
    let head = format!(
        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n",
        WRAPPED_ERROR_BODY.len()
    );
    let (addr, accepted) =
        one_shot_server([head.as_bytes(), WRAPPED_ERROR_BODY.as_bytes()].concat(), 3);
    let p = OpenRouterProvider::new("k".into())
        .with_endpoint(format!("http://{addr}/v1/chat/completions"));
    let (on_chunk, chunks) = collecting_chunk_fn();
    let req = CompletionRequest {
        model: "m".into(),
        system: None,
        prompt: "hi".into(),
        max_tokens: 8,
    };
    let res = tokio::time::timeout(Duration::from_secs(10), p.complete_streaming(req, on_chunk))
        .await
        .expect("must not hang");
    assert!(
        matches!(res, Err(ProviderError::Api(_))),
        "wrapped 200 error body must surface as an error, not a fabricated empty success; got {res:?}"
    );
    assert!(chunks.lock().unwrap().is_empty());
    assert_eq!(
        accepted.load(Ordering::SeqCst),
        3,
        "must have retried through MAX_RETRIES before giving up"
    );
}

// --- Important-2 / Important-3: leftover carry at EOF + UTF-8 codepoints
// split across TCP chunks ----------------------------------------------------

#[tokio::test]
async fn reassembles_line_split_mid_multibyte_across_writes_with_crlf_and_no_final_newline() {
    use tokio::io::AsyncWriteExt;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let listener = tokio::net::TcpListener::from_std(listener).unwrap();

    // CRLF line endings. The second delta line is split so the write
    // boundary falls inside "é"'s two-byte UTF-8 encoding (0xC3 0xA9). The
    // final usage frame has no trailing newline at all — the stream just
    // ends (pins the leftover-carry fix, Important-2).
    let line1 = "data: {\"choices\":[{\"delta\":{\"content\":\"Hi \"}}]}\r\n";
    let line2 = "data: {\"choices\":[{\"delta\":{\"content\":\"h\u{e9}llo\"}}]}\r\n";
    let line3 = "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":2}}";

    let e_byte_pos = line2.find('\u{e9}').expect("contains é");
    let split_at = e_byte_pos + 1; // right after é's first byte (0xC3)
    let (line2_head, line2_tail) = line2.as_bytes().split_at(split_at);

    let total_len = line1.len() + line2.len() + line3.len();
    let head = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {total_len}\r\n\r\n"
    );

    tokio::spawn(async move {
        let Ok((mut sock, _)) = listener.accept().await else {
            return;
        };
        let _ = read_request(&mut sock).await; // drain the client's request
        let _ = sock.write_all(head.as_bytes()).await;
        let _ = sock.write_all(line1.as_bytes()).await;
        tokio::time::sleep(Duration::from_millis(5)).await;
        let _ = sock.write_all(line2_head).await;
        tokio::time::sleep(Duration::from_millis(5)).await;
        let _ = sock.write_all(line2_tail).await;
        tokio::time::sleep(Duration::from_millis(5)).await;
        let _ = sock.write_all(line3.as_bytes()).await;
        let _ = sock.shutdown().await;
    });

    let p = OpenRouterProvider::new("k".into())
        .with_endpoint(format!("http://{addr}/v1/chat/completions"));
    let (on_chunk, chunks) = collecting_chunk_fn();
    let req = CompletionRequest {
        model: "m".into(),
        system: None,
        prompt: "hi".into(),
        max_tokens: 8,
    };
    let done = tokio::time::timeout(Duration::from_secs(5), p.complete_streaming(req, on_chunk))
        .await
        .expect("must not hang")
        .unwrap();
    assert_eq!(chunks.lock().unwrap().concat(), "Hi h\u{e9}llo");
    assert_eq!(done.text, "Hi h\u{e9}llo");
    assert_eq!(
        done.input_tokens, 3,
        "usage frame captured: prompt_tokens=3"
    );
    assert_eq!(
        done.output_tokens, 2,
        "usage frame captured: completion_tokens=2"
    );
}

#[tokio::test]
async fn missing_usage_frame_falls_back_to_chars_over_4_estimate() {
    // No `usage` field anywhere in the stream (as if the endpoint silently
    // ignores `stream_options.include_usage`) — the final Completion must
    // still carry a usable (if approximate) token count, not an error.
    let sse_body = "data: {\"choices\":[{\"delta\":{\"content\":\"12345678\"}}]}\ndata: [DONE]\n";
    let head = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\n\r\n",
        sse_body.len()
    );
    let (addr, _accepted) = one_shot_server([head.as_bytes(), sse_body.as_bytes()].concat(), 1);
    let p = OpenRouterProvider::new("k".into())
        .with_endpoint(format!("http://{addr}/v1/chat/completions"));
    let (on_chunk, _chunks) = collecting_chunk_fn();
    let req = CompletionRequest {
        model: "m".into(),
        system: None,
        prompt: "abcdefgh".into(), // 8 chars → 2 estimated input tokens
        max_tokens: 8,
    };
    let done = p.complete_streaming(req, on_chunk).await.unwrap();
    assert_eq!(done.text, "12345678");
    assert_eq!(done.output_tokens, 2, "8 chars / 4 = 2");
    assert_eq!(done.input_tokens, 2, "prompt is 8 chars / 4 = 2");
}

// --- Important-4 / Minor-7: estimate unit symmetry + tiny-reply floor ------

#[tokio::test]
async fn missing_usage_estimate_uses_chars_not_bytes_and_floors_tiny_output_at_one() {
    // CJK text: each char is 3 bytes in UTF-8. Before the fix, input tokens
    // were estimated from byte length (`str::len()`) while output tokens
    // used char count (`chars().count()`) — a 3x divergence on CJK. Both
    // must now agree on chars()/4. The reply is a single character, which
    // chars/4 truncates to 0 — Minor-7 floors any non-empty output estimate
    // at 1.
    let reply = "文"; // 1 char, 3 bytes
    let sse_body = format!(
        "data: {{\"choices\":[{{\"delta\":{{\"content\":\"{reply}\"}}}}]}}\ndata: [DONE]\n"
    );
    let head = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\n\r\n",
        sse_body.len()
    );
    let (addr, _accepted) = one_shot_server([head.as_bytes(), sse_body.as_bytes()].concat(), 1);
    let p = OpenRouterProvider::new("k".into())
        .with_endpoint(format!("http://{addr}/v1/chat/completions"));
    let (on_chunk, _chunks) = collecting_chunk_fn();
    let prompt = "文文文文"; // 4 chars, 12 bytes: chars/4 = 1, bytes/4 = 3
    let req = CompletionRequest {
        model: "m".into(),
        system: None,
        prompt: prompt.into(),
        max_tokens: 8,
    };
    let done = p.complete_streaming(req, on_chunk).await.unwrap();
    assert_eq!(done.text, reply);
    assert_eq!(done.input_tokens, 1, "4 CJK chars / 4 = 1, not bytes/4 = 3");
    assert_eq!(
        done.output_tokens, 1,
        "1 non-empty char floors to 1 (Minor-7), not chars/4 = 0"
    );
}

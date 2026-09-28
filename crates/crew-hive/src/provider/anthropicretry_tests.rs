//! Retry and decoding over a socket: a busy answer is tried again, a bad
//! request is not, a stream that has shown text is never replayed, and a
//! character split between two reads arrives whole.
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type Reply = Vec<Vec<u8>>; // one response, as the writes that make it up

const OVERLOADED: &str =
    r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#;
const LIMITED: &str = r#"{"type":"error","error":{"type":"rate_limit_error","message":"slow"}}"#;
const BAD: &str = r#"{"type":"error","error":{"type":"invalid_request_error","message":"no"}}"#;
const OK: &str = r#"{"type":"message","content":[{"type":"text","text":"ok"}],"usage":{"input_tokens":1,"output_tokens":1}}"#;

/// A loopback server answering the Nth request with `replies[N]`, flushing
/// and pausing after each write so the client reads them apart. Returns the
/// base URL and how many requests arrived.
fn serve(replies: Vec<Reply>) -> (String, Arc<AtomicUsize>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
    let asked = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&asked);
    tokio::spawn(async move {
        for reply in replies {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            read_request(&mut sock).await;
            count.fetch_add(1, Ordering::SeqCst);
            for part in reply {
                let _ = sock.write_all(&part).await;
                let _ = sock.flush().await;
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            let _ = sock.shutdown().await;
        }
    });
    (format!("http://{addr}"), asked)
}

/// The whole request — headers, then `content-length` bytes — before any
/// answer: answering early races the client still writing it.
async fn read_request(sock: &mut tokio::net::TcpStream) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = sock.read(&mut chunk).await.unwrap_or(0);
        if n == 0 {
            return;
        }
        buf.extend_from_slice(&chunk[..n]);
        let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
            continue;
        };
        let head = String::from_utf8_lossy(&buf[..end]).to_lowercase();
        let len: usize = head
            .split("content-length:")
            .nth(1)
            .and_then(|s| s.lines().next())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        if buf.len() >= end + 4 + len {
            return;
        }
    }
}

/// A whole JSON response; `headers` are extra lines, each ending `\r\n`.
fn whole(status: &str, headers: &str, body: &str) -> Reply {
    let head = format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\n{headers}content-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    );
    vec![[head.as_bytes(), body.as_bytes()].concat()]
}

/// A 200 event stream, one HTTP chunk per part: the client gets each chunk
/// as its own read, so a part boundary inside a character is a split read.
fn events(parts: &[&[u8]]) -> Reply {
    let head = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n";
    let mut w = vec![head.as_bytes().to_vec()];
    for p in parts {
        w.push([format!("{:x}\r\n", p.len()).as_bytes(), p, b"\r\n"].concat());
    }
    w.push(b"0\r\n\r\n".to_vec());
    w
}

fn delta(text: &str) -> String {
    format!("event: content_block_delta\ndata: {{\"type\":\"content_block_delta\",\"delta\":{{\"type\":\"text_delta\",\"text\":\"{text}\"}}}}\n\n")
}

fn error_event() -> String {
    format!("event: error\ndata: {OVERLOADED}\n\n")
}

fn req() -> CompletionRequest {
    CompletionRequest {
        model: "claude-x".into(),
        prompt: "hi".into(),
        max_tokens: 16,
        ..Default::default()
    }
}

/// The call's result and the text fragments it handed the pane.
async fn stream(base: &str) -> (Result<Completion, ProviderError>, Vec<String>) {
    let got = Arc::new(Mutex::new(Vec::<String>::new()));
    let g = Arc::clone(&got);
    let on_chunk: ChunkFn = Arc::new(move |c: crate::provider::Chunk<'_>| {
        if let crate::provider::Chunk::Text(s) = c {
            g.lock().unwrap().push(s.to_string());
        }
    });
    let p = AnthropicProvider::new("k".into()).with_base_url(base);
    let fut = p.complete_streaming(req(), on_chunk);
    let res = tokio::time::timeout(Duration::from_secs(10), fut).await;
    let chunks = got.lock().unwrap().clone();
    (res.expect("must not hang"), chunks)
}

async fn whole_call(base: &str) -> Result<Completion, ProviderError> {
    let fut = AnthropicProvider::new("k".into())
        .with_base_url(base)
        .complete(req());
    let res = tokio::time::timeout(Duration::from_secs(10), fut).await;
    res.expect("must not hang")
}

#[tokio::test]
async fn an_overloaded_529_is_tried_again() {
    let first = whole("529 Overloaded", "retry-after: 0\r\n", OVERLOADED);
    let (base, asked) = serve(vec![first, whole("200 OK", "", OK)]);
    assert_eq!(whole_call(&base).await.unwrap().text, "ok");
    assert_eq!(asked.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_rate_limited_429_waits_as_told_and_is_tried_again() {
    let first = whole("429 Too Many Requests", "retry-after: 0\r\n", LIMITED);
    let (base, asked) = serve(vec![first, whole("200 OK", "", OK)]);
    assert_eq!(whole_call(&base).await.unwrap().text, "ok");
    assert_eq!(asked.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_bad_request_is_not_tried_again() {
    let first = whole("400 Bad Request", "retry-after: 0\r\n", BAD);
    let (base, asked) = serve(vec![first, whole("200 OK", "", OK)]);
    let res = whole_call(&base).await;
    assert!(matches!(res, Err(ProviderError::Api(ref b)) if b.contains("invalid_request_error")));
    assert_eq!(asked.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_stream_refused_with_529_before_any_byte_is_tried_again() {
    let first = whole("529 Overloaded", "retry-after: 0\r\n", OVERLOADED);
    let (base, asked) = serve(vec![first, events(&[delta("ok").as_bytes()])]);
    let (res, chunks) = stream(&base).await;
    assert_eq!(res.unwrap().text, "ok");
    assert_eq!(chunks, ["ok"]);
    assert_eq!(asked.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn an_overload_event_before_any_text_is_tried_again() {
    let first = events(&[error_event().as_bytes()]);
    let (base, asked) = serve(vec![first, events(&[delta("ok").as_bytes()])]);
    let (res, chunks) = stream(&base).await;
    assert_eq!(res.unwrap().text, "ok");
    assert_eq!(chunks, ["ok"]);
    assert_eq!(asked.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn an_overload_after_text_was_shown_is_not_replayed() {
    let first = events(&[delta("par").as_bytes(), error_event().as_bytes()]);
    let (base, asked) = serve(vec![first, events(&[delta("par").as_bytes()])]);
    let (res, chunks) = stream(&base).await;
    assert!(matches!(res, Err(ProviderError::Api(ref e)) if e.contains("overloaded_error")));
    assert_eq!(chunks, ["par"], "shown once, never typed out again");
    assert_eq!(asked.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_character_split_between_two_reads_arrives_whole() {
    let line = delta("a—b");
    let at = line.find('—').unwrap() + 1; // after the first of its 3 bytes
    let (head, tail) = line.as_bytes().split_at(at);
    let (base, _) = serve(vec![events(&[head, tail])]);
    let (res, chunks) = stream(&base).await;
    let text = res.unwrap().text;
    assert_eq!(text, "a—b");
    assert!(!chunks.concat().contains('\u{FFFD}'), "{chunks:?}");
}

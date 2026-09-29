//! The streamed path end to end over a socket: the request asks for a
//! stream, and the reply's fragments reach the callback as they arrive.
use super::*;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const EVENTS: &str = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":9}}}\n\n\
event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"two \"}}\n\n\
event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"parts\"}}\n\n\
event: message_delta\ndata: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":2}}\n\n";

#[tokio::test]
async fn a_text_reply_streams_fragment_by_fragment() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let asked = Arc::new(Mutex::new(String::new()));
    let seen = Arc::clone(&asked);
    tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 16_384];
        let n = sock.read(&mut buf).await.unwrap();
        *seen.lock().unwrap() = String::from_utf8_lossy(&buf[..n]).to_string();
        let head =
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n";
        sock.write_all(head.as_bytes()).await.unwrap();
        for part in EVENTS.split_inclusive("\n\n") {
            sock.write_all(part.as_bytes()).await.unwrap();
            sock.flush().await.unwrap();
        }
        let _ = sock.shutdown().await;
    });
    let p = AnthropicProvider::new("k".into()).with_base_url(&format!("http://{addr}"));
    let got = Arc::new(Mutex::new(Vec::<String>::new()));
    let g = Arc::clone(&got);
    let on_chunk: ChunkFn = Arc::new(move |c: crate::provider::Chunk<'_>| {
        if let crate::provider::Chunk::Text(s) = c {
            g.lock().unwrap().push(s.to_string());
        }
    });
    let req = CompletionRequest {
        model: "claude-x".into(),
        prompt: "hi".into(),
        max_tokens: 16,
        ..Default::default()
    };
    let c = p.complete_streaming(req, on_chunk).await.unwrap();
    assert_eq!(c.text, "two parts");
    assert_eq!((c.input_tokens, c.output_tokens), (9, 2));
    assert_eq!(
        got.lock().unwrap().clone(),
        vec!["two ".to_string(), "parts".to_string()]
    );
    assert!(
        asked.lock().unwrap().contains("\"stream\":true"),
        "asked for a stream"
    );
    assert!(
        !asked.lock().unwrap().contains("cache_control"),
        "a one-shot pays no cache-write surcharge (`anthropiccache`)"
    );
}

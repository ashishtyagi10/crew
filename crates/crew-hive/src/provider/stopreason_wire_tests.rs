//! Over a socket: the OpenAI-shaped stream's closing `finish_reason`, and
//! the user turn the relay sends after a cut reply, as each provider puts it
//! on the wire.
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::provider::{ChunkFn, Completion, CompletionRequest, OpenRouterProvider, Provider, Turn};

/// One connection: read the whole request (answering early races the client
/// still writing it), keep it, answer with `sse` as an event stream.
fn serve(sse: String) -> (SocketAddr, Arc<Mutex<String>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
    let asked = Arc::new(Mutex::new(String::new()));
    let kept = Arc::clone(&asked);
    tokio::spawn(async move {
        let Ok((mut sock, _)) = listener.accept().await else {
            return;
        };
        let (mut buf, mut chunk) = (Vec::new(), [0u8; 8192]);
        loop {
            let n = sock.read(&mut chunk).await.unwrap_or(0);
            buf.extend_from_slice(&chunk[..n]);
            let text = String::from_utf8_lossy(&buf).into_owned();
            let whole = text.find("\r\n\r\n").is_some_and(|end| {
                let len = text[..end]
                    .to_lowercase()
                    .split("content-length:")
                    .nth(1)
                    .and_then(|s| s.split("\r\n").next())
                    .and_then(|s| s.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                buf.len() >= end + 4 + len
            });
            if n == 0 || whole {
                *kept.lock().unwrap() = text;
                break;
            }
        }
        let head = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\n\r\n",
            sse.len()
        );
        let _ = sock.write_all(head.as_bytes()).await;
        let _ = sock.write_all(sse.as_bytes()).await;
        let _ = sock.shutdown().await;
    });
    (addr, asked)
}

/// A real chat-completions stream: text chunks with a null reason, then the
/// choice's closing chunk carrying `finish_reason`, then the usage chunk.
fn stream(finish_reason: &str) -> String {
    let chunk = |choice: &str| {
        format!("data: {{\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{choice}]}}\n\n")
    };
    [
        chunk(r#"{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}"#),
        chunk(r#"{"index":0,"delta":{"content":"The answer is "},"finish_reason":null}"#),
        chunk(r#"{"index":0,"delta":{"content":"forty"},"finish_reason":null}"#),
        chunk(&format!(r#"{{"index":0,"delta":{{}},"finish_reason":"{finish_reason}"}}"#)),
        "data: {\"id\":\"chatcmpl-1\",\"choices\":[],\"usage\":{\"prompt_tokens\":12,\"completion_tokens\":2048}}\n\n".into(),
        "data: [DONE]\n\n".into(),
    ]
    .concat()
}

async fn streamed(sse: String, turns: Vec<Turn>) -> (Completion, String) {
    let (addr, asked) = serve(sse);
    let p = OpenRouterProvider::new("k".into())
        .with_endpoint(format!("http://{addr}/v1/chat/completions"));
    let quiet: ChunkFn = Arc::new(|_| {});
    let req = CompletionRequest {
        model: "m".into(),
        prompt: "what is six times seven?".into(),
        max_tokens: 2048,
        turns,
        ..Default::default()
    };
    let c = p.complete_streaming(req, quiet).await.unwrap();
    let asked = asked.lock().unwrap().clone();
    (c, asked)
}

#[tokio::test]
async fn an_openai_stream_whose_last_choice_stops_on_length_is_truncated() {
    let (c, _) = streamed(stream("length"), Vec::new()).await;
    assert_eq!(c.text, "The answer is forty");
    assert!(
        c.truncated,
        "the closing finish_reason \"length\" is the ceiling"
    );
}

#[tokio::test]
async fn an_openai_stream_that_stops_on_its_own_is_not() {
    let (c, _) = streamed(stream("stop"), Vec::new()).await;
    assert_eq!(c.text, "The answer is forty");
    assert!(!c.truncated);
}

fn continued() -> Vec<Turn> {
    vec![
        Turn::Assistant {
            text: "The answer is forty".into(),
            calls: Vec::new(),
        },
        Turn::User("Continue exactly where it stopped.".into()),
    ]
}

#[tokio::test]
async fn a_user_turn_goes_out_after_the_models_as_a_user_message() {
    let (_, asked) = streamed(stream("stop"), continued()).await;
    let body = &asked[asked.find("\r\n\r\n").expect("a request") + 4..];
    let v: serde_json::Value = serde_json::from_str(body).unwrap();
    let roles: Vec<&str> = v["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["role"].as_str().unwrap())
        .collect();
    assert_eq!(roles, ["user", "assistant", "user"]);
    assert_eq!(v["messages"][1]["content"], "The answer is forty");
    assert_eq!(
        v["messages"][2]["content"],
        "Continue exactly where it stopped."
    );
}

#[test]
fn anthropic_and_the_cli_carry_the_user_turn_too() {
    let req = CompletionRequest {
        prompt: "what is six times seven?".into(),
        turns: continued(),
        ..Default::default()
    };
    let m = crate::provider::anthropic::build_messages(&req);
    assert_eq!(m.len(), 3);
    assert_eq!(m[2]["role"], "user");
    assert_eq!(m[2]["content"], "Continue exactly where it stopped.");
    let flat = crate::provider::claudecli::prompt_text(&req);
    let (said, asked) = (
        flat.find("The answer is forty").unwrap(),
        flat.find("[user]\nContinue exactly where it stopped.")
            .unwrap(),
    );
    assert!(said < asked, "{flat}");
}

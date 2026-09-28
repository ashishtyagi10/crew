//! A fallback chain is billed at the model that answered, not the one asked
//! for first.
use std::sync::{Arc, Mutex};

use super::tests::read_request;
use super::OpenRouterProvider;
use crate::graph::ModelTier;
use crate::provider::{CompletionRequest, Provider};

/// A loopback server that answers its Nth connection with `replies[N]`,
/// keeping each request's `model` so the test can see the chain was walked.
fn scripted(replies: Vec<String>) -> (std::net::SocketAddr, Arc<Mutex<Vec<String>>>) {
    use tokio::io::AsyncWriteExt;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
    let asked = Arc::new(Mutex::new(Vec::new()));
    let log = asked.clone();
    tokio::spawn(async move {
        for reply in replies {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let req = read_request(&mut sock).await;
            let body = String::from_utf8_lossy(&req);
            let json = body.split("\r\n\r\n").nth(1).unwrap_or("");
            let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
            log.lock()
                .unwrap()
                .push(v["model"].as_str().unwrap_or("").to_string());
            let _ = sock.write_all(reply.as_bytes()).await;
            let _ = sock.shutdown().await;
        }
    });
    (addr, asked)
}

fn http(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// DashScope's default chain: `qwen-max` 404s (a region that does not serve
/// it), `qwen-plus` answers — with no `model` in its body, so the attempt's
/// own model is the one that answered. 1000 in + 100 out at qwen-plus's
/// $0.4/$1.2 per M is 400 + 120 µ$; at qwen-max's $1.6/$6.4 it would read
/// 1600 + 640.
#[tokio::test]
async fn a_fallback_reply_is_billed_at_the_model_that_answered() {
    let answer = r#"{"choices":[{"message":{"content":"hi"}}],
        "usage":{"prompt_tokens":1000,"completion_tokens":100}}"#;
    let (addr, asked) = scripted(vec![
        http(
            "404 Not Found",
            r#"{"error":{"message":"The model `qwen-max` does not exist"}}"#,
        ),
        http("200 OK", answer),
    ]);
    let p = OpenRouterProvider::new("k".into())
        .with_endpoint(format!("http://{addr}/v1/chat/completions"))
        .with_fallbacks(vec!["qwen-plus".into()]);
    let req = CompletionRequest {
        model: "qwen-max".into(),
        prompt: "hi".into(),
        max_tokens: 8,
        ..Default::default()
    };
    let c = p.complete(req).await.expect("the fallback answers");
    assert_eq!(asked.lock().unwrap().as_slice(), ["qwen-max", "qwen-plus"]);
    assert_eq!(c.model, "qwen-plus");
    let billed = crate::apiagent::billed("qwen-max", ModelTier::Standard, &c);
    assert_eq!(billed, 400 + 120, "priced at qwen-plus");
    assert_ne!(billed, 1600 + 640, "priced at qwen-max, which 404ed");
}

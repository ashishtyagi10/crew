use super::*;
use crate::provider::{CompletionRequest, OpenRouterProvider, Provider, ToolDef};

const DASHSCOPE: &str = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions";

fn body() -> Value {
    json!({"model": "qwen3.8-max", "max_tokens": 4096, "messages": []})
}

#[test]
fn a_text_protocol_body_stops_at_the_closing_tag() {
    let b = super::super::for_endpoint(DASHSCOPE, body());
    assert_eq!(b["stop"], json!(["</tool_call>"]));
    assert_eq!(b["max_tokens"], 4096, "the ceiling keeps its name here");
}

#[test]
fn native_tools_openai_s_host_and_a_stop_already_set_are_left_alone() {
    let mut tools = body();
    tools["tools"] = json!([{"type": "function"}]);
    assert!(super::super::for_endpoint(DASHSCOPE, tools)
        .get("stop")
        .is_none());
    let openai = super::super::for_endpoint("https://api.openai.com/v1/chat/completions", body());
    assert!(openai.get("stop").is_none(), "{openai}");
    let mut set = body();
    set["stop"] = json!(["END"]);
    assert_eq!(
        super::super::for_endpoint(DASHSCOPE, set)["stop"],
        json!(["END"])
    );
}

/// What the provider actually sends: one request's JSON body, answered "ok".
async fn sent(req: CompletionRequest) -> Value {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "http://{}/v1/chat/completions",
        listener.local_addr().unwrap()
    );
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let (mut buf, mut chunk) = (Vec::new(), [0u8; 4096]);
        let body = loop {
            let n = sock.read(&mut chunk).await.unwrap();
            buf.extend_from_slice(&chunk[..n]);
            let text = String::from_utf8_lossy(&buf).to_string();
            let json = text.split("\r\n\r\n").nth(1).unwrap_or("");
            if let Ok(v) = serde_json::from_str::<Value>(json) {
                break v;
            }
        };
        let ok = r#"{"choices":[{"message":{"content":"ok"}}],"usage":{"prompt_tokens":1,"completion_tokens":1}}"#;
        let reply = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{ok}",
            ok.len()
        );
        sock.write_all(reply.as_bytes()).await.unwrap();
        body
    });
    OpenRouterProvider::new("k".into())
        .with_endpoint(url)
        .complete(req)
        .await
        .unwrap();
    server.await.unwrap()
}

#[tokio::test]
async fn the_wire_carries_the_stop_only_when_calls_are_text() {
    let text = CompletionRequest {
        model: "qwen3.8-max".into(),
        prompt: "hi".into(),
        max_tokens: 8,
        ..Default::default()
    };
    assert_eq!(sent(text.clone()).await["stop"], json!(["</tool_call>"]));
    let tool = ToolDef {
        name: "sys__run".into(),
        description: String::new(),
        input_schema: json!({"type": "object"}),
    };
    let native = CompletionRequest {
        tools: vec![tool],
        ..text
    };
    assert!(sent(native).await.get("stop").is_none());
}

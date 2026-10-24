//! Over a socket, from the stream's first frame to the next request: a call
//! whose arguments are not JSON is answered, not run.
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::refusal;
use crate::agent::{Agent, AgentContext};
use crate::bus::{AgentId, EventBus};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::{OpenRouterProvider, ToolInvocation};
use crate::tools::{ToolSpec, Tools};

/// One connection per answer, in order; each request is kept whole.
/// `connection: close`, so the client never reuses a socket this has shut.
fn serve(answers: Vec<String>) -> (SocketAddr, Arc<Mutex<Vec<String>>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
    let asked = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&asked);
    tokio::spawn(async move {
        for sse in answers {
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
                    kept.lock().unwrap().push(text);
                    break;
                }
            }
            let head = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\n\
                 connection: close\r\ncontent-length: {}\r\n\r\n",
                sse.len()
            );
            let _ = sock.write_all(head.as_bytes()).await;
            let _ = sock.write_all(sse.as_bytes()).await;
            let _ = sock.shutdown().await;
        }
    });
    (addr, asked)
}

/// A stream whose one call to `fs__read` carries `args`, split across two
/// frames as a real stream splits it, then closes on `finish_reason`.
fn calling(args: &str, finish_reason: &str) -> String {
    let (a, b) = args.split_at(args.len() / 2);
    let frame = |delta: Value, why: Value| {
        let choice = json!({"index": 0, "delta": delta, "finish_reason": why});
        format!("data: {}\n\n", json!({"choices": [choice]}))
    };
    let first =
        json!({"index": 0, "id": "call_1", "function": {"name": "fs__read", "arguments": a}});
    let rest = json!({"index": 0, "function": {"arguments": b}});
    [
        frame(json!({"tool_calls": [first]}), Value::Null),
        frame(json!({"tool_calls": [rest]}), Value::Null),
        frame(json!({}), json!(finish_reason)),
        "data: [DONE]\n\n".into(),
    ]
    .concat()
}

fn answering(text: &str) -> String {
    let choice = json!({"index": 0, "delta": {"content": text}, "finish_reason": "stop"});
    format!("data: {}\n\ndata: [DONE]\n\n", json!({"choices": [choice]}))
}

/// Counts every call it is asked to run.
struct Reader(Arc<Mutex<Vec<String>>>);

impl Tools for Reader {
    fn hint(&self) -> String {
        String::new()
    }
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        self.0
            .lock()
            .unwrap()
            .push(format!("{server}:{tool} {args}"));
        Ok("fn main() {}".into())
    }
    fn specs(&self) -> Vec<ToolSpec> {
        vec![ToolSpec {
            server: "fs".into(),
            tool: "read".into(),
            description: "read a file".into(),
            input_schema: json!({"type": "object"}),
        }]
    }
}

/// Run one task against `answers` over the wire: what the tools were asked
/// to run, and the second request's `tool` message as the model is shown it.
async fn run(answers: Vec<String>) -> (Vec<String>, String) {
    let (addr, asked) = serve(answers);
    let provider = OpenRouterProvider::new("k".into())
        .with_endpoint(format!("http://{addr}/v1/chat/completions"));
    let ran = Arc::new(Mutex::new(Vec::new()));
    let bus = EventBus::new(64);
    let task = TaskSpec {
        id: TaskId(1),
        title: "t".into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: vec![],
        prompt: "read a.rs".into(),
        specialty: String::new(),
        expertise: String::new(),
    };
    let ctx = AgentContext {
        cancel: Default::default(),
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(1),
        task,
        deps: vec![],
        bus: bus.clone(),
    };
    let out = crate::apiagent::ApiAgent::new(Arc::new(provider), 64)
        .with_tools(Arc::new(Reader(Arc::clone(&ran))))
        .run(ctx)
        .await;
    assert!(out.success, "{}", out.output);
    let asked = asked.lock().unwrap().clone();
    assert_eq!(asked.len(), 2, "one call, then the follow-up");
    let body = &asked[1][asked[1].find("\r\n\r\n").unwrap() + 4..];
    let body: Value = serde_json::from_str(body).unwrap();
    let tool = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("the call is answered");
    assert_eq!(tool["tool_call_id"], "call_1", "paired to its call");
    let ran = ran.lock().unwrap().clone();
    (ran, tool["content"].as_str().unwrap().to_string())
}

#[tokio::test]
async fn a_call_cut_off_at_the_token_limit_is_answered_not_run() {
    let cut = calling(r#"{"path": "a.rs""#, "length");
    let (ran, said) = run(vec![cut, answering("ok")]).await;
    assert!(ran.is_empty(), "a call with no arguments was run: {ran:?}");
    assert!(
        said.starts_with("ERROR: your arguments were not valid JSON: EOF while parsing"),
        "{said}"
    );
    assert!(said.contains(r#"— {"path": "a.rs""#), "{said}");
    assert!(said.contains("cut off at the token limit"), "{said}");
}

#[tokio::test]
async fn a_trailing_comma_is_forgiven_and_the_call_runs() {
    let sloppy = calling(r#"{"path":"a.rs",}"#, "tool_calls");
    let (ran, said) = run(vec![sloppy, answering("ok")]).await;
    assert_eq!(ran, [r#"fs:read {"path":"a.rs"}"#]);
    assert_eq!(said, "fn main() {}");
}

#[test]
fn a_bad_call_not_cut_off_says_nothing_of_the_limit_and_a_good_one_runs() {
    let mut call = ToolInvocation {
        id: "c".into(),
        name: "fs__read".into(),
        input: json!({}),
        bad_args: None,
    };
    assert_eq!(refusal(&call, true), None, "read arguments run");
    call.bad_args = Some("expected `:` at line 1 column 8 \u{2014} {'path'".into());
    let o = refusal(&call, false).unwrap();
    assert!(o.is_error);
    assert_eq!(
        o.content,
        "your arguments were not valid JSON: expected `:` at line 1 column 8 \u{2014} {'path'"
    );
}

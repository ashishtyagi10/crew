//! The router's verdict and the agent's own word, as the level the work
//! turn's request carries — down to the bytes a fake server receives on
//! each wire.
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Duration;

use super::*;
use crate::broker::adapter::Adapter;
use crate::broker::apiadapter::ApiAdapter;
use crate::broker::intent::decision::{Decision, Routing};
use serde_json::Value;

fn routed(shape: Shape) -> Routing {
    Routing::Chosen(Decision {
        shape,
        ..Default::default()
    })
}

#[test]
fn judgment_routes_think_high_restating_ones_low_and_the_rest_at_the_default() {
    assert_eq!(of_route(Shape::Plan), Some(Effort::High));
    assert_eq!(of_route(Shape::Review), Some(Effort::High));
    assert_eq!(of_route(Shape::Commit), Some(Effort::Low));
    assert_eq!(of_route(Shape::Standup), Some(Effort::Low));
    for open in [
        Shape::Reply,
        Shape::Fan,
        Shape::Loop,
        Shape::Goal,
        Shape::Swarm,
    ] {
        assert_eq!(of_route(open), None, "{open:?}");
    }
}

#[test]
fn the_route_s_level_lasts_as_long_as_the_routed_message() {
    let empty = std::env::temp_dir().join(format!("crew-re-none-{}", std::process::id()));
    {
        let (d, _scope) = decided(&routed(Shape::Review));
        assert_eq!(d.shape, Shape::Review);
        assert_eq!(for_turn_at(&empty, "coder"), Some(Effort::High));
        {
            let _inner = enter(None);
            assert_eq!(for_turn_at(&empty, "coder"), None);
        }
        assert_eq!(for_turn_at(&empty, "coder"), Some(Effort::High), "restored");
    }
    assert_eq!(
        for_turn_at(&empty, "coder"),
        None,
        "the next message sends none"
    );
}

/// A project whose store holds `coder` at `effort`.
fn store(effort: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!(
        "crew-re-{}-{}",
        std::process::id(),
        effort.replace(' ', "_")
    ));
    std::fs::create_dir_all(base.join(".crew")).unwrap();
    let json = format!(r#"[{{"name":"coder","role":"","last_used":1,"effort":"{effort}"}}]"#);
    std::fs::write(base.join(".crew").join("specialists.json"), json).unwrap();
    base
}

#[test]
fn an_agent_s_own_level_outranks_the_route_and_a_typo_does_not() {
    let _route = enter(Some(Effort::Low));
    assert_eq!(for_turn_at(&store("max"), "coder"), Some(Effort::Max));
    assert_eq!(for_turn_at(&store("max"), "writer"), Some(Effort::Low));
    assert_eq!(for_turn_at(&store("hard"), "coder"), Some(Effort::Low));
}

#[test]
fn a_rewritten_store_keeps_the_agent_s_level() {
    let base = store("high");
    super::super::specialists::record_at(&base, &[("coder".into(), "x".into())]);
    let kept = super::super::specialists::load_at(&base);
    assert_eq!(
        kept[0].effort.as_ref().and_then(|v| v.as_str()),
        Some("high")
    );
}

/// One request's JSON body, answered 401 so nothing retries or falls back.
fn fake() -> (String, std::thread::JoinHandle<Value>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut sock, _) = listener.accept().unwrap();
        let (mut buf, mut chunk) = (Vec::new(), [0u8; 8192]);
        let body = loop {
            let n = sock.read(&mut chunk).unwrap();
            buf.extend_from_slice(&chunk[..n]);
            let text = String::from_utf8_lossy(&buf).to_string();
            let json = text.split("\r\n\r\n").nth(1).unwrap_or("");
            if let Ok(v) = serde_json::from_str::<Value>(json) {
                break v;
            }
        };
        let no = r#"{"error":{"type":"authentication_error","message":"no"}}"#;
        let reply = format!(
            "HTTP/1.1 401 Unauthorized\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{no}",
            no.len()
        );
        let _ = sock.write_all(reply.as_bytes());
        body
    });
    (url, server)
}

/// What an agent named `name` on `model` sends, on the Anthropic wire.
fn anthropic(name: &str, model: &str) -> Value {
    let (url, server) = fake();
    let provider = crew_hive::AnthropicProvider::new("k".into()).with_base_url(&url);
    let a = ApiAdapter::new(name, model, "", None, Arc::new(provider)).unwrap();
    let _ = a.call("the diff", Duration::from_secs(10));
    server.join().unwrap()
}

/// The same on the chat-completions wire.
fn openai(model: &str) -> Value {
    let (url, server) = fake();
    let provider = crew_hive::OpenRouterProvider::new("k".into())
        .with_endpoint(format!("{url}/v1/chat/completions"));
    let a = ApiAdapter::new("coder", model, "", None, Arc::new(provider)).unwrap();
    let _ = a.call("the diff", Duration::from_secs(10));
    server.join().unwrap()
}

#[test]
fn the_anthropic_wire_carries_the_route_s_level() {
    let _env = crate::broker::testenv::mock("unused");
    let level = |shape: Shape, model: &str| {
        let (_d, _scope) = decided(&routed(shape));
        anthropic("coder", model).get("output_config").cloned()
    };
    let effort = |e: &str| Some(serde_json::json!({ "effort": e }));
    assert_eq!(level(Shape::Commit, "claude-opus-5-5"), effort("low"));
    assert_eq!(level(Shape::Plan, "claude-opus-5-5"), effort("high"));
    assert_eq!(level(Shape::Reply, "claude-opus-5-5"), None);
    assert_eq!(
        level(Shape::Plan, "claude-haiku-4-5"),
        None,
        "no effort there"
    );
}

#[test]
fn an_agent_set_to_max_sends_max_whatever_the_route() {
    let env = crate::broker::testenv::mock("unused");
    let dir = std::path::PathBuf::from(std::env::var("CREW_PROJECT_DIR").unwrap());
    std::fs::create_dir_all(dir.join(".crew")).unwrap();
    // Not `coder`: the hop tests run beside this one, unlocked, and must
    // never read this store's level for their agent.
    let json = r#"[{"name":"deep-thinker","role":"","last_used":1,"effort":"max"}]"#;
    std::fs::write(dir.join(".crew").join("specialists.json"), json).unwrap();
    let (_d, _scope) = decided(&routed(Shape::Commit));
    let body = anthropic("deep-thinker", "claude-opus-5-5");
    assert_eq!(body["output_config"]["effort"], "max", "{body}");
    drop(env);
}

#[test]
fn the_chat_completions_wire_carries_it_to_gpt_6_only() {
    let _env = crate::broker::testenv::mock("unused");
    let (_d, _scope) = decided(&routed(Shape::Review));
    let sol = openai("gpt-6.1-sol");
    assert_eq!(sol["reasoning_effort"], "high", "{sol}");
    let old = openai("gpt-4.1");
    assert!(old.get("reasoning_effort").is_none(), "{old}");
}

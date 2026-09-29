use super::super::{AnthropicProvider, ToolDef, Turn};
use super::*;

fn one_shot() -> CompletionRequest {
    CompletionRequest {
        model: "claude-sonnet-5-5".into(),
        system: Some("be brief".into()),
        prompt: "hi".into(),
        max_tokens: 16,
        ..Default::default()
    }
}

/// A tool loop's first round: tools, no turns yet — the next round replays
/// this whole prefix.
fn tool_round() -> CompletionRequest {
    CompletionRequest {
        tools: vec![ToolDef {
            name: "weather__current".into(),
            description: "current conditions".into(),
            input_schema: serde_json::json!({"type": "object"}),
        }],
        ..one_shot()
    }
}

/// A cutoff continuation: turns, no tools, sent once and never followed.
fn follow_up() -> CompletionRequest {
    CompletionRequest {
        turns: vec![
            Turn::Assistant {
                text: "hello".into(),
                calls: Vec::new(),
            },
            Turn::User("and goodbye?".into()),
        ],
        ..one_shot()
    }
}

/// The field is automatic caching's, so it sits at the TOP level of the
/// body — never on a content block, where it would be an explicit
/// breakpoint pinned to one spot instead of one that moves with the loop.
fn asks(body: &serde_json::Value) -> bool {
    let top = body.get("cache_control") == Some(&serde_json::json!({"type": "ephemeral"}));
    let nested = ["messages", "tools", "system"]
        .iter()
        .any(|k| body[*k].to_string().contains("cache_control"));
    assert!(!nested, "cache_control on a block: {body:#}");
    top
}

#[test]
fn a_tool_round_asks_for_the_cache_at_the_top_level() {
    let body = AnthropicProvider::body_if(true, &tool_round(), false);
    assert!(asks(&body), "{body:#}");
    assert!(body["tools"].is_array(), "tools still ride beside it");
}

#[test]
fn a_streamed_tool_round_asks_but_a_tool_free_follow_up_does_not() {
    let streamed = AnthropicProvider::body_if(true, &tool_round(), true);
    assert!(asks(&streamed), "{streamed:#}");
    assert_eq!(streamed["stream"], true);
    for stream in [false, true] {
        let body = AnthropicProvider::body_if(true, &follow_up(), stream);
        assert!(body.get("cache_control").is_none(), "{body:#}");
    }
    assert!(!caches(true, &follow_up()));
}

/// A one-shot's prefix is never sent again: asking would only ever write,
/// at 1.25× input, and never read it back.
#[test]
fn a_one_shot_never_asks_streamed_or_not() {
    for stream in [false, true] {
        let body = AnthropicProvider::body_if(true, &one_shot(), stream);
        assert!(body.get("cache_control").is_none(), "{body:#}");
    }
    assert!(!caches(true, &one_shot()));
}

#[test]
fn crew_anthropic_cache_zero_turns_it_off() {
    assert!(!enabled_from(Some("0")));
    assert!(enabled_from(None), "on by default");
    assert!(enabled_from(Some("1")));
    assert!(!caches(false, &tool_round()));
    assert!(!caches(false, &follow_up()));
    for stream in [false, true] {
        let body = AnthropicProvider::body_if(false, &follow_up(), stream);
        assert!(body.get("cache_control").is_none(), "{body:#}");
    }
    let body = AnthropicProvider::body_if(false, &tool_round(), false);
    assert!(body.get("cache_control").is_none(), "{body:#}");
}

/// Over a socket: a tool round's request reaches the wire with the field at
/// the top level of its JSON — or, on a machine that set
/// `CREW_ANTHROPIC_CACHE=0`, without it.
#[tokio::test]
async fn a_tool_rounds_request_carries_the_field_on_the_wire() {
    use super::super::anthropic::retry_tests::{read_request, whole};
    use super::super::Provider;
    use tokio::io::AsyncWriteExt;
    const OK: &str = r#"{"type":"message","content":[{"type":"text","text":"ok"}],"usage":{"input_tokens":1,"output_tokens":1}}"#;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let raw = read_request(&mut sock).await;
        for part in whole("200 OK", "", OK) {
            sock.write_all(&part).await.unwrap();
        }
        let _ = sock.shutdown().await;
        raw
    });
    let p = AnthropicProvider::new("k".into()).with_base_url(&format!("http://{addr}"));
    assert_eq!(p.complete(tool_round()).await.unwrap().text, "ok");
    let raw = String::from_utf8(server.await.unwrap()).unwrap();
    let (_, json) = raw
        .split_once("\r\n\r\n")
        .expect("a body after the headers");
    let sent: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(asks(&sent), enabled(), "{sent:#}");
    assert!(sent["tools"].is_array());
}

/// The body sent is the one the switch in the environment decides — the
/// wiring, checked whatever this machine's `CREW_ANTHROPIC_CACHE` says.
#[test]
fn the_sent_body_reads_the_switch_from_the_environment() {
    for stream in [false, true] {
        assert_eq!(
            AnthropicProvider::body(&tool_round(), stream),
            AnthropicProvider::body_if(enabled(), &tool_round(), stream)
        );
    }
}

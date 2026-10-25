//! Native tool use on the wire, and where a request to Anthropic goes.
use super::*;
use crate::provider::AnthropicProvider;

fn tool_req() -> CompletionRequest {
    CompletionRequest {
        model: "m".into(),
        system: Some("be brief".into()),
        prompt: "weather in Oslo?".into(),
        max_tokens: 100,
        tools: vec![ToolDef {
            name: "weather__current".into(),
            description: "current conditions".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"q": {"type": "string"}},
                "required": ["q"],
            }),
        }],
        turns: vec![
            Turn::Assistant {
                text: "checking".into(),
                calls: vec![ToolInvocation {
                    id: "call_1".into(),
                    name: "weather__current".into(),
                    input: serde_json::json!({"q": "Oslo"}),
                    bad_args: None,
                }],
            },
            Turn::ToolResults(vec![ToolOutcome {
                id: "call_1".into(),
                name: "weather__current".into(),
                content: "4C clear".into(),
                is_error: false,
            }]),
        ],
    }
}

#[test]
fn anthropic_sends_the_schema_not_a_description_clip() {
    let tools = super::anthropic::build_tools(&tool_req()).expect("tools present");
    let t = &tools[0];
    assert_eq!(t["name"], "weather__current");
    // The argument SHAPE reaches the model — the whole reason for this path.
    assert_eq!(t["input_schema"]["properties"]["q"]["type"], "string");
    assert_eq!(t["input_schema"]["required"][0], "q");
}

#[test]
fn anthropic_omits_tools_entirely_when_there_are_none() {
    // Absent, not `[]`: an empty array is a different request and some
    // endpoints reject it.
    let plain = CompletionRequest {
        model: "m".into(),
        prompt: "hi".into(),
        max_tokens: 10,
        ..Default::default()
    };
    assert!(super::anthropic::build_tools(&plain).is_none());
}

#[test]
fn anthropic_replays_the_tool_use_block_and_pairs_the_result_by_id() {
    let m = super::anthropic::build_messages(&tool_req());
    assert_eq!(m.len(), 3, "user, assistant, tool-result user: {m:#?}");
    // The assistant's call must be replayed, or the result's id has nothing
    // to pair with and the API rejects the request.
    let call = &m[1]["content"][1];
    assert_eq!(call["type"], "tool_use");
    assert_eq!(call["id"], "call_1");
    assert_eq!(call["input"]["q"], "Oslo");
    // Results come back as a USER turn.
    assert_eq!(m[2]["role"], "user");
    assert_eq!(m[2]["content"][0]["type"], "tool_result");
    assert_eq!(m[2]["content"][0]["tool_use_id"], "call_1");
    assert_eq!(m[2]["content"][0]["is_error"], false);
}

#[test]
fn anthropic_parses_tool_use_blocks_and_joins_every_text_block() {
    let body = r#"{
      "content": [
        {"type":"text","text":"let me check. "},
        {"type":"tool_use","id":"toolu_9","name":"weather__current","input":{"q":"Oslo"}},
        {"type":"text","text":"one moment"}
      ],
      "usage": {"input_tokens": 10, "output_tokens": 4}
    }"#;
    let c = AnthropicProvider::parse_response(body).unwrap();
    assert_eq!(c.text, "let me check. one moment");
    assert_eq!(c.calls.len(), 1);
    assert_eq!(c.calls[0].id, "toolu_9");
    assert_eq!(c.calls[0].name, "weather__current");
    assert_eq!(c.calls[0].input["q"], "Oslo");
}

#[test]
fn anthropic_a_reply_with_no_tools_still_parses_with_no_calls() {
    let body = r#"{"content":[{"type":"text","text":"hi"}],
                   "usage":{"input_tokens":1,"output_tokens":1}}"#;
    let c = AnthropicProvider::parse_response(body).unwrap();
    assert_eq!(c.text, "hi");
    assert!(c.calls.is_empty());
}

#[test]
fn the_openai_shape_serialises_arguments_as_a_string_both_ways() {
    let body = r#"{
      "choices":[{"message":{"content":null,"tool_calls":[
        {"id":"call_7","type":"function",
         "function":{"name":"weather__current","arguments":"{\"q\":\"Oslo\"}"}}
      ]}}],
      "usage":{"prompt_tokens":5,"completion_tokens":2}
    }"#;
    // `content: null` is what a tool-only reply sends; parsing must survive it.
    let c = super::openai_http::parse_response(body).unwrap();
    assert_eq!(c.text, "");
    assert_eq!(c.calls.len(), 1);
    assert_eq!(c.calls[0].id, "call_7");
    assert_eq!(c.calls[0].input["q"], "Oslo");
}

#[test]
fn the_openai_shape_survives_unparseable_arguments() {
    let body = r#"{
      "choices":[{"message":{"content":null,"tool_calls":[
        {"id":"c","type":"function","function":{"name":"n","arguments":""}}
      ]}}],
      "usage":{"prompt_tokens":1,"completion_tokens":1}
    }"#;
    // Losing the whole completion because a model emitted `""` would be worse
    // than letting the tool reject an empty object.
    let c = super::openai_http::parse_response(body).unwrap();
    assert_eq!(c.calls[0].input, serde_json::json!({}));
}

/// The two credential forms send different headers: a key rides in
/// `x-api-key` (what every install has always sent); an OAuth bearer (the
/// Anthropic CLI's minted profile token) rides as `Authorization: Bearer`
/// AND the OAuth beta header, without which `/v1/messages` refuses it.
#[test]
fn anthropic_auth_headers_follow_the_credential_form() {
    let key = AnthropicProvider::new("sk-ant-api03-k".into());
    assert_eq!(
        key.auth_headers(),
        vec![("x-api-key", "sk-ant-api03-k".to_string())]
    );
    let oauth = AnthropicProvider::with_oauth("sk-ant-oat01-t".into());
    assert_eq!(
        oauth.auth_headers(),
        vec![
            ("authorization", "Bearer sk-ant-oat01-t".to_string()),
            ("anthropic-beta", "oauth-2025-04-20".to_string()),
        ]
    );
}

/// `ANTHROPIC_BASE_URL` rebases the endpoint onto `<base>/v1/messages`,
/// tolerant of the two ways a base is usually spelled.
#[test]
fn anthropic_base_url_seam_targets_v1_messages() {
    use super::anthropic::messages_url;
    let live = AnthropicProvider::new("k".into());
    assert_eq!(live.endpoint(), "https://api.anthropic.com/v1/messages");
    let stub = AnthropicProvider::new("k".into()).with_base_url("http://127.0.0.1:9/");
    assert_eq!(stub.endpoint(), "http://127.0.0.1:9/v1/messages");
    assert_eq!(messages_url("http://h/v1"), "http://h/v1/messages");
    assert_eq!(messages_url("http://h"), "http://h/v1/messages");
}

use super::*;
use crate::provider::AnthropicProvider;

fn test_request() -> CompletionRequest {
    CompletionRequest {
        model: "m".into(),
        system: None,
        prompt: "one two three".into(),
        max_tokens: 100,
        ..Default::default()
    }
}

#[tokio::test]
async fn mock_streams_reply_in_chunks_then_completes() {
    let p = MockProvider {
        reply: "alpha beta gamma delta".to_string(),
    };
    let chunks = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let sink = chunks.clone();
    let on_chunk: ChunkFn = std::sync::Arc::new(move |c: Chunk<'_>| {
        if let Chunk::Text(s) = c {
            sink.lock().unwrap().push(s.to_string());
        }
    });
    let done = p
        .complete_streaming(test_request(), on_chunk)
        .await
        .unwrap();
    let got = chunks.lock().unwrap();
    assert!(
        got.len() >= 2,
        "reply arrives in at least 2 chunks: {got:?}"
    );
    assert_eq!(
        got.concat(),
        "alpha beta gamma delta",
        "chunks reassemble the reply"
    );
    assert_eq!(done.text, "alpha beta gamma delta");
}

#[tokio::test]
async fn default_streaming_falls_back_without_chunks() {
    // Any provider using the trait default must behave like complete().
    // MockProvider OVERRIDES it, so exercise the default through a tiny
    // local test provider that only implements `complete`.
    struct Plain;
    impl Provider for Plain {
        fn complete(
            &self,
            _req: CompletionRequest,
        ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
            Box::pin(async {
                Ok(Completion {
                    text: "whole".into(),
                    input_tokens: 1,
                    output_tokens: 1,
                    cost_microusd: 0,
                    ..Default::default()
                })
            })
        }
    }
    let ticked = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let t = ticked.clone();
    let on_chunk: ChunkFn = std::sync::Arc::new(move |_| {
        t.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    });
    let done = Plain
        .complete_streaming(test_request(), on_chunk)
        .await
        .unwrap();
    assert_eq!(done.text, "whole");
    assert_eq!(
        ticked.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "default never chunks"
    );
}

#[tokio::test]
async fn mock_provider_echoes_reply_and_counts() {
    let p = MockProvider {
        reply: "hello there".into(),
    };
    let c = p
        .complete(CompletionRequest {
            model: "m".into(),
            system: None,
            prompt: "one two three".into(),
            max_tokens: 100,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(c.text, "hello there");
    assert_eq!(c.input_tokens, 3);
    assert_eq!(c.output_tokens, 2);
}

#[test]
fn provider_is_object_safe() {
    let _p: Box<dyn Provider> = Box::new(MockProvider { reply: "x".into() });
}

#[test]
fn parse_response_extracts_text_and_usage() {
    let body = r#"{
        "content": [{"type": "text", "text": "Hello world"}],
        "usage": {"input_tokens": 12, "output_tokens": 5},
        "stop_reason": "end_turn"
    }"#;
    let c = AnthropicProvider::parse_response(body).unwrap();
    assert_eq!(c.text, "Hello world");
    assert_eq!(c.input_tokens, 12);
    assert_eq!(c.output_tokens, 5);
}

/// A `thinking` block is kept as the completion's thought — never as text.
#[test]
fn parse_response_keeps_a_thinking_block_as_thought_not_text() {
    let body = r#"{
        "content": [
            {"type": "thinking", "thinking": "let me see", "signature": "abc"},
            {"type": "text", "text": "Hello world"}
        ],
        "usage": {"input_tokens": 12, "output_tokens": 5}
    }"#;
    let c = AnthropicProvider::parse_response(body).unwrap();
    assert_eq!(c.text, "Hello world");
    assert_eq!(c.thought, "let me see");
}

/// The mock reasons when its reply says so, and the reasoning is not text.
#[tokio::test]
async fn mock_routes_a_think_fence_to_thought_before_the_text() {
    let p = MockProvider {
        reply: "<think>weigh it</think>alpha beta".to_string(),
    };
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(bool, String)>::new()));
    let sink = seen.clone();
    let on_chunk: ChunkFn = std::sync::Arc::new(move |c: Chunk<'_>| {
        sink.lock().unwrap().push(match c {
            Chunk::Text(s) => (false, s.to_string()),
            Chunk::Thought(s) => (true, s.to_string()),
        });
    });
    let done = p
        .complete_streaming(test_request(), on_chunk)
        .await
        .unwrap();
    let got = seen.lock().unwrap();
    assert_eq!(
        got[0],
        (true, "weigh it".to_string()),
        "thought first: {got:?}"
    );
    assert!(got[1..].iter().all(|(t, _)| !t), "then only text");
    assert_eq!(done.text, "alpha beta");
    assert_eq!(done.thought, "weigh it");
    assert_eq!(
        done.output_tokens, 2,
        "tokens count the reply, not the working"
    );
}

#[test]
fn parse_response_errors_on_api_error_payload() {
    let body = r#"{"type":"error","error":{"type":"overloaded_error","message":"overloaded"}}"#;
    assert!(matches!(
        AnthropicProvider::parse_response(body),
        Err(ProviderError::Api(_))
    ));
}

/// The rule, tested against a value rather than the process environment.
///
/// This used to read `ANTHROPIC_API_KEY` and assert only when it was absent,
/// so on any machine that had one it asserted NOTHING and passed — silently,
/// and on exactly the machines most likely to run it. It also never covered
/// the empty-string case, which is the one that matters: an exported but
/// blank key is not a key.
#[test]
fn a_missing_or_blank_anthropic_key_errors() {
    assert!(matches!(
        AnthropicProvider::from_key(None),
        Err(ProviderError::MissingKey("ANTHROPIC_API_KEY"))
    ));
    assert!(matches!(
        AnthropicProvider::from_key(Some(String::new())),
        Err(ProviderError::MissingKey("ANTHROPIC_API_KEY"))
    ));
    assert!(AnthropicProvider::from_key(Some("sk-x".into())).is_ok());
}

#[test]
fn openrouter_parse_response_extracts_text_and_usage() {
    use super::openai_http::parse_response;
    // OpenAI-shaped response: choices[].message.content + usage token names.
    let body = r#"{
        "choices": [{"message": {"role": "assistant", "content": "Hello world"}}],
        "usage": {"prompt_tokens": 7, "completion_tokens": 3}
    }"#;
    let c = parse_response(body).unwrap();
    assert_eq!(c.text, "Hello world");
    assert_eq!(c.input_tokens, 7);
    assert_eq!(c.output_tokens, 3);
}

#[test]
fn openrouter_parse_response_errors_on_api_error_payload() {
    use super::openai_http::parse_response;
    let body = r#"{"error":{"code":402,"message":"insufficient credits"}}"#;
    assert!(matches!(parse_response(body), Err(ProviderError::Api(_))));
}

#[test]
fn parse_response_reads_openrouter_cost() {
    use super::openai_http::parse_response;
    let body = r#"{"choices":[{"message":{"content":"hi"}}],
        "usage":{"prompt_tokens":10,"completion_tokens":5,"cost":0.000129}}"#;
    let c = parse_response(body).unwrap();
    assert_eq!(c.cost_microusd, 129);
}

#[test]
fn parse_response_without_cost_is_zero() {
    use super::openai_http::parse_response;
    let body = r#"{"choices":[{"message":{"content":"hi"}}],
        "usage":{"prompt_tokens":10,"completion_tokens":5}}"#;
    assert_eq!(parse_response(body).unwrap().cost_microusd, 0);
}

#[test]
fn sse_parser_extracts_deltas_usage_and_done() {
    use super::openai_http::{parse_sse_frame, SseItem};
    let none: Vec<SseItem> = Vec::new();
    assert_eq!(parse_sse_frame(""), none);
    assert_eq!(parse_sse_frame(": keep-alive"), none);
    assert_eq!(parse_sse_frame("data: [DONE]"), vec![SseItem::Done]);
    assert_eq!(
        parse_sse_frame(r#"data: {"choices":[{"delta":{"content":"hel"}}]}"#),
        vec![SseItem::Delta("hel".into())]
    );
    // Role-only first frame: no content → nothing, not an error.
    assert_eq!(
        parse_sse_frame(r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#),
        none
    );
    // Usage frame (stream_options include_usage / final frame).
    assert_eq!(
        parse_sse_frame(
            r#"data: {"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":42}}"#,
        ),
        vec![SseItem::Usage(10, 42, 0)]
    );
    // OpenRouter usage frame with exact cost (dollars, converted to micro-USD).
    assert_eq!(
        parse_sse_frame(
            r#"data: {"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":42,"cost":0.000129}}"#,
        ),
        vec![SseItem::Usage(10, 42, 129)]
    );
    assert_eq!(parse_sse_frame("data: {not json"), none);
}

#[test]
fn a_missing_or_blank_openrouter_key_errors() {
    assert!(matches!(
        OpenRouterProvider::from_key(None),
        Err(ProviderError::MissingKey("OPENROUTER_API_KEY"))
    ));
    assert!(matches!(
        OpenRouterProvider::from_key(Some(String::new())),
        Err(ProviderError::MissingKey("OPENROUTER_API_KEY"))
    ));
    assert!(OpenRouterProvider::from_key(Some("sk-x".into())).is_ok());
}

#[tokio::test]
#[ignore = "requires ANTHROPIC_API_KEY; run with --ignored"]
async fn live_anthropic_completion() {
    let p = AnthropicProvider::from_env().expect("key");
    let c = p
        .complete(CompletionRequest {
            model: "claude-haiku-4-5".into(),
            system: Some("Reply with exactly the word: pong".into()),
            prompt: "ping".into(),
            max_tokens: 16,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(!c.text.is_empty());
    assert!(c.output_tokens > 0);
}

#[tokio::test]
async fn arc_dyn_provider_is_a_provider() {
    // The broker holds Arc<dyn Provider>; LlmPlanner<P: Provider> must accept it.
    fn takes_provider<P: crate::provider::Provider>(p: P) -> P {
        p
    }
    let arc: std::sync::Arc<dyn crate::provider::Provider> =
        std::sync::Arc::new(crate::provider::MockProvider { reply: "ok".into() });
    let p = takes_provider(arc);
    let got = p
        .complete(crate::provider::CompletionRequest {
            model: "mock".into(),
            system: None,
            prompt: "hi".into(),
            max_tokens: 16,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(got.text, "ok");
}

/// The message names the variable that was actually missing. It used to say
/// `ANTHROPIC_API_KEY` whatever had failed — and `OpenRouterProvider` backs
/// six providers through six different variables, so it was usually wrong.
#[test]
fn a_missing_key_names_its_own_variable() {
    assert_eq!(
        ProviderError::MissingKey("OPENAI_API_KEY").to_string(),
        "OPENAI_API_KEY not set"
    );
    assert_eq!(
        ProviderError::MissingKey("ANTHROPIC_API_KEY").to_string(),
        "ANTHROPIC_API_KEY not set"
    );
}

// ---------------------------------------------------------------------------
// Native tool use — request mapping and response parsing
// ---------------------------------------------------------------------------

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

/// One request is one `claude -p` run: print mode, JSON out, Claude Code's
/// own tools OFF (this is a model, not an agent), no session left on disk,
/// the request's model, and the system prompt only when there is one.
#[test]
fn claude_cli_args_are_the_documented_headless_contract() {
    use crate::provider::ClaudeCliProvider;
    let mut req = test_request();
    req.model = "claude-sonnet-4-6".into();
    let args = ClaudeCliProvider::args(&req);
    assert_eq!(
        args,
        [
            "-p",
            "one two three",
            "--output-format",
            "json",
            "--tools",
            "",
            "--no-session-persistence",
            "--model",
            "claude-sonnet-4-6",
        ]
    );
    req.system = Some("be terse".into());
    let args = ClaudeCliProvider::args(&req);
    assert_eq!(&args[args.len() - 2..], ["--system-prompt", "be terse"]);
    req.system = Some("   ".into());
    assert!(!ClaudeCliProvider::args(&req).contains(&"--system-prompt".to_string()));
    assert!(!ClaudeCliProvider::new().supports_tools());
}

/// A multi-turn request (the native tool loop's shape) flattens into one
/// prompt that ends on an assistant cue; a one-shot request is untouched.
#[test]
fn claude_cli_flattens_turns_into_one_prompt() {
    use crate::provider::claudecli::prompt_text;
    let mut req = test_request();
    assert_eq!(prompt_text(&req), "one two three");
    req.turns = vec![
        Turn::Assistant {
            text: "looking".into(),
            calls: vec![ToolInvocation {
                id: "c1".into(),
                name: "read".into(),
                input: serde_json::json!({"path": "x"}),
            }],
        },
        Turn::ToolResults(vec![ToolOutcome {
            id: "c1".into(),
            name: "read".into(),
            content: "contents".into(),
            is_error: false,
        }]),
    ];
    let p = prompt_text(&req);
    assert!(
        p.starts_with("one two three\n\n[assistant]\nlooking\n@read {\"path\":\"x\"}"),
        "{p}"
    );
    assert!(p.contains("[tool results]\nc1: contents"), "{p}");
    assert!(p.ends_with("[assistant]\n"), "{p}");
}

/// The envelope `claude -p --output-format json` printed on 2026-09-09
/// (claude 2.x, haiku): `result` is the reply, usage rides along, cost is
/// NOT reported (the plan covers it), and the CLI's own errors surface as
/// an API error carrying the sentence.
#[test]
fn claude_cli_reads_the_json_envelope() {
    use crate::provider::ClaudeCliProvider;
    let ok = r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":1500,
        "result":"pong","session_id":"f8713122-408e-4df0-a10b-9384744a4b26",
        "total_cost_usd":0.0027815,"num_turns":1,
        "usage":{"input_tokens":10,"cache_read_input_tokens":25215,"output_tokens":50},
        "permission_denials":[],"terminal_reason":"completed"}"#;
    let c = ClaudeCliProvider::parse_result(ok).unwrap();
    assert_eq!(
        (c.text.as_str(), c.input_tokens, c.output_tokens),
        ("pong", 10, 50)
    );
    assert_eq!(c.cost_microusd, 0);
    assert!(c.calls.is_empty());

    let refused = r#"{"type":"result","subtype":"error_during_execution","is_error":true,
        "result":"Not logged in \u00b7 run claude auth login"}"#;
    match ClaudeCliProvider::parse_result(refused) {
        Err(ProviderError::Api(body)) => assert!(body.contains("Not logged in"), "{body}"),
        other => panic!("{other:?}"),
    }
    let bare = r#"{"type":"result","subtype":"error_max_turns","is_error":true}"#;
    match ClaudeCliProvider::parse_result(bare) {
        Err(ProviderError::Api(body)) => assert!(body.contains("error_max_turns"), "{body}"),
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        ClaudeCliProvider::parse_result("not json at all"),
        Err(ProviderError::Decode(_))
    ));
}

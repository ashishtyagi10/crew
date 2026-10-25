//! `claude -p` as a provider: the argv, the prompt, the envelope, the stream.
use super::tests::test_request;
use super::*;

/// One request is one `claude -p` run: print mode, the STREAMED JSON
/// envelope (`--verbose` is what lets `-p` stream; partial messages carry
/// the deltas), Claude Code's own tools OFF and the person's MCP servers
/// with them (this is a model, not an agent), no session left on disk, the
/// request's model, and the system prompt only when there is one.
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
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--tools",
            "",
            "--strict-mcp-config",
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
                bad_args: None,
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
    // Plain text is the reply of last resort (a CLI without the stream,
    // or a test's fake that echoes) — never a decode error.
    assert_eq!(
        ClaudeCliProvider::parse_result("not json at all")
            .unwrap()
            .text,
        "not json at all"
    );
    assert!(matches!(
        ClaudeCliProvider::parse_result("   \n"),
        Err(ProviderError::Api(_))
    ));
}

/// The live run: a fake `claude` that prints the stream one line at a time
/// — a status line, a thinking delta, two text deltas, the closing
/// `result`. Every delta reaches `on_chunk` in its lane, and the completion
/// carries the settled reply, the whole thought, and the usage.
#[cfg(unix)]
#[test]
fn claude_cli_streams_thinking_and_text_then_settles_on_the_result() {
    use crate::provider::ClaudeCliProvider;
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("crew-claudecli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let fake = dir.join("claude");
    std::fs::write(
        &fake,
        concat!(
            "#!/bin/sh\n",
            "echo '{\"type\":\"system\",\"subtype\":\"status\",\"status\":\"requesting\"}'\n",
            "echo '{\"type\":\"stream_event\",\"event\":{\"type\":\"content_block_delta\",\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"weigh it\"}}}'\n",
            "echo '{\"type\":\"stream_event\",\"event\":{\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"po\"}}}'\n",
            "echo '{\"type\":\"stream_event\",\"event\":{\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"ng\"}}}'\n",
            "echo '{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"pong\",\"usage\":{\"input_tokens\":3,\"output_tokens\":4}}'\n",
        ),
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let log = seen.clone();
    let on_chunk: ChunkFn = std::sync::Arc::new(move |c: Chunk<'_>| {
        log.lock().unwrap().push(match c {
            Chunk::Text(t) => format!("text:{t}"),
            Chunk::Thought(t) => format!("thought:{t}"),
        });
    });
    let rt = tokio::runtime::Runtime::new().unwrap();
    let p = ClaudeCliProvider::new().with_program(fake.to_str().unwrap());
    let c = rt
        .block_on(p.complete_streaming(test_request(), on_chunk))
        .unwrap();
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        ["thought:weigh it", "text:po", "text:ng"]
    );
    assert_eq!((c.text.as_str(), c.thought.as_str()), ("pong", "weigh it"));
    assert_eq!(
        (c.input_tokens, c.output_tokens, c.cost_microusd),
        (3, 4, 0)
    );
    // A CLI that prints plain text instead of the stream still answers.
    std::fs::write(&fake, "#!/bin/sh\necho 'plain answer'\n").unwrap();
    let c = rt.block_on(p.complete(test_request())).unwrap();
    assert_eq!(c.text, "plain answer");
    let _ = std::fs::remove_dir_all(&dir);
}

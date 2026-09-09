//! The lines are what `claude` 2.1.267 printed on 2026-09-09 for
//! `-p … --output-format stream-json --verbose --include-partial-messages`,
//! trimmed of ids; the thinking, tool-use and tool-result shapes follow the
//! same message format (Anthropic content blocks inside `message.content`).
use super::{parse_line, Fold, StreamEvent};

const TEXT: &str = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"ello from stream"}},"session_id":"f0e5","parent_tool_use_id":null,"uuid":"2e7e"}"#;
const THINK: &str = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Let me look"}}}"#;
const START: &str = r#"{"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}},"session_id":"f0e5"}"#;
const ASSISTANT: &str = r#"{"type":"assistant","message":{"model":"claude-sonnet-5","id":"msg_1","type":"message","role":"assistant","content":[{"type":"text","text":"hello from stream"}],"stop_reason":null},"session_id":"f0e5"}"#;
const TOOL_USE: &str = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_1","name":"Read","input":{"file_path":"/x.rs"}}]}}"#;
const TOOL_RESULT: &str = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_1","content":[{"type":"text","text":"fn main() {}"}],"is_error":false}]}}"#;
const STATUS: &str = r#"{"type":"system","subtype":"status","status":"requesting","session_id":"f0e5","uuid":"a218"}"#;
const HOOK: &str = r#"{"type":"system","subtype":"hook_started","hook_id":"9dcc","hook_name":"SessionStart:startup","hook_event":"SessionStart","uuid":"3e96","session_id":"f0e5"}"#;
const RATE: &str = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","rateLimitType":"five_hour"}}"#;
const RESULT: &str = r#"{"duration_api_ms":1318,"stop_reason":"end_turn","session_id":"f0e5","total_cost_usd":0.16472199999999998,"usage":{"input_tokens":2,"cache_creation_input_tokens":41162,"cache_read_input_tokens":0,"output_tokens":7},"type":"result","subtype":"success","is_error":false,"duration_ms":2000,"num_turns":1,"result":"hello from stream"}"#;

#[test]
fn deltas_status_and_noise_parse_to_what_crew_acts_on() {
    assert_eq!(
        parse_line(TEXT),
        Some(StreamEvent::Text("ello from stream".into()))
    );
    assert_eq!(
        parse_line(THINK),
        Some(StreamEvent::Thought("Let me look".into()))
    );
    assert_eq!(parse_line(START), Some(StreamEvent::Other));
    assert_eq!(
        parse_line(STATUS),
        Some(StreamEvent::Status("requesting".into()))
    );
    assert_eq!(parse_line(HOOK), Some(StreamEvent::Other));
    assert_eq!(parse_line(RATE), Some(StreamEvent::Other));
    assert_eq!(parse_line(""), None);
    assert_eq!(parse_line("Not logged in"), None);
    assert_eq!(parse_line("{not json"), None);
}

#[test]
fn tool_use_and_its_result_carry_the_call_id_arguments_and_text() {
    assert_eq!(
        parse_line(TOOL_USE),
        Some(StreamEvent::ToolUse {
            id: "toolu_1".into(),
            name: "Read".into(),
            input: r#"{"file_path":"/x.rs"}"#.into(),
        })
    );
    assert_eq!(
        parse_line(TOOL_RESULT),
        Some(StreamEvent::ToolResult {
            id: "toolu_1".into(),
            ok: true,
            text: "fn main() {}".into(),
        })
    );
    let err = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t2","content":"no such file","is_error":true}]}}"#;
    assert_eq!(
        parse_line(err),
        Some(StreamEvent::ToolResult {
            id: "t2".into(),
            ok: false,
            text: "no such file".into(),
        })
    );
}

#[test]
fn the_result_line_carries_reply_usage_and_cost() {
    let Some(StreamEvent::Result(r)) = parse_line(RESULT) else {
        panic!("not a result");
    };
    assert_eq!(r.text, "hello from stream");
    assert!(!r.is_error);
    assert_eq!(r.subtype, "success");
    assert_eq!((r.input_tokens, r.output_tokens), (2, 7));
    assert_eq!(r.cost_microusd, 164_722);
}

/// The reply is the `result` line's text; the deltas are only the live
/// view of it. A whole assistant text block counts only when no delta
/// streamed (a CLI without `--include-partial-messages`), so nothing is
/// doubled.
#[test]
fn fold_prefers_the_result_then_the_deltas_and_never_doubles_text() {
    let mut f = Fold::default();
    for l in [STATUS, THINK, TEXT, ASSISTANT, RESULT] {
        f.push(parse_line(l).as_ref(), l);
    }
    assert_eq!(f.text, "ello from stream");
    assert_eq!(f.thought, "Let me look");
    assert_eq!(f.reply(), "hello from stream");

    let mut no_result = Fold::default();
    for l in [TEXT] {
        no_result.push(parse_line(l).as_ref(), l);
    }
    assert_eq!(no_result.reply(), "ello from stream");

    let mut whole_only = Fold::default();
    whole_only.push(parse_line(ASSISTANT).as_ref(), ASSISTANT);
    assert_eq!(whole_only.reply(), "hello from stream");
}

/// A CLI (or a test's fake) that prints plain text instead of the stream
/// still yields a reply: the raw lines, trimmed.
#[test]
fn fold_keeps_raw_lines_as_the_reply_of_last_resort() {
    let mut f = Fold::default();
    for l in ["claude here", "", "second line  "] {
        f.push(parse_line(l).as_ref(), l);
    }
    assert!(f.result.is_none());
    assert_eq!(f.reply(), "claude here\nsecond line");
}

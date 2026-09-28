//! Each provider's own wire shape for "the token ceiling ended this reply",
//! next to the shape for "the model finished", so a parser that ignores the
//! field fails the first test of its pair and one that reads it too eagerly
//! fails the second.
use crate::provider::anthropicsse::Fold;
use crate::provider::openai_http::parse_response;
use crate::provider::{AnthropicProvider, ChunkFn, ClaudeCliProvider};
use std::sync::Arc;

fn openai_body(finish_reason: &str) -> String {
    format!(
        r#"{{"id":"chatcmpl-1","object":"chat.completion","model":"qwen-max",
        "choices":[{{"index":0,"message":{{"role":"assistant","content":"The answer is forty"}},
        "finish_reason":"{finish_reason}"}}],
        "usage":{{"prompt_tokens":12,"completion_tokens":2048,"total_tokens":2060}}}}"#
    )
}

#[test]
fn an_openai_reply_stopped_by_length_is_truncated() {
    let c = parse_response(&openai_body("length")).unwrap();
    assert_eq!(c.text, "The answer is forty");
    assert!(c.truncated, "finish_reason \"length\" is the token ceiling");
}

#[test]
fn an_openai_reply_that_stopped_on_its_own_is_not() {
    assert!(!parse_response(&openai_body("stop")).unwrap().truncated);
    assert!(
        !parse_response(&openai_body("tool_calls"))
            .unwrap()
            .truncated
    );
    // An endpoint that sends no finish_reason at all: finished, as before.
    let bare = r#"{"choices":[{"message":{"content":"hi"}}],"usage":{"prompt_tokens":1,"completion_tokens":1}}"#;
    assert!(!parse_response(bare).unwrap().truncated);
}

fn anthropic_body(stop_reason: &str) -> String {
    format!(
        r#"{{"id":"msg_01","type":"message","role":"assistant","model":"claude-sonnet-5",
        "content":[{{"type":"text","text":"half a sen"}}],
        "stop_reason":"{stop_reason}","stop_sequence":null,
        "usage":{{"input_tokens":12,"output_tokens":16}}}}"#
    )
}

#[test]
fn an_anthropic_reply_stopped_by_max_tokens_is_truncated() {
    let c = AnthropicProvider::parse_response(&anthropic_body("max_tokens")).unwrap();
    assert_eq!(c.text, "half a sen");
    assert!(
        c.truncated,
        "stop_reason \"max_tokens\" is the token ceiling"
    );
}

#[test]
fn an_anthropic_reply_that_ended_its_turn_is_not() {
    let c = AnthropicProvider::parse_response(&anthropic_body("end_turn")).unwrap();
    assert!(!c.truncated);
    let c = AnthropicProvider::parse_response(&anthropic_body("tool_use")).unwrap();
    assert!(!c.truncated);
}

/// The Messages stream as the API sends it: the stop reason rides on
/// `message_delta`, after the last text delta.
fn anthropic_stream(stop_reason: &str) -> String {
    format!(
        "event: message_start\n\
data: {{\"type\":\"message_start\",\"message\":{{\"id\":\"msg_01\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"stop_reason\":null,\"usage\":{{\"input_tokens\":12,\"output_tokens\":1}}}}}}\n\n\
event: content_block_start\n\
data: {{\"type\":\"content_block_start\",\"index\":0,\"content_block\":{{\"type\":\"text\",\"text\":\"\"}}}}\n\n\
event: content_block_delta\n\
data: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{{\"type\":\"text_delta\",\"text\":\"half a sen\"}}}}\n\n\
event: content_block_stop\n\
data: {{\"type\":\"content_block_stop\",\"index\":0}}\n\n\
event: message_delta\n\
data: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"{stop_reason}\",\"stop_sequence\":null}},\"usage\":{{\"output_tokens\":16}}}}\n\n\
event: message_stop\n\
data: {{\"type\":\"message_stop\"}}\n\n"
    )
}

fn fold(stream: &str) -> crate::provider::Completion {
    let quiet: ChunkFn = Arc::new(|_| {});
    let mut f = Fold::default();
    for piece in stream.as_bytes().chunks(11) {
        f.feed(std::str::from_utf8(piece).unwrap(), &quiet);
    }
    f.finish(&quiet).unwrap()
}

#[test]
fn an_anthropic_stream_stopped_by_max_tokens_is_truncated() {
    let c = fold(&anthropic_stream("max_tokens"));
    assert_eq!(c.text, "half a sen");
    assert!(
        c.truncated,
        "message_delta.delta.stop_reason is the ceiling"
    );
}

#[test]
fn an_anthropic_stream_that_ended_its_turn_is_not() {
    assert!(!fold(&anthropic_stream("end_turn")).truncated);
}

/// The CLI's closing line, as `claude` 2.x prints it (see `claudestream_tests`).
fn cli_result(stop_reason: &str) -> String {
    format!(
        r#"{{"type":"result","subtype":"success","is_error":false,"duration_ms":2000,"num_turns":1,"result":"half a sen","stop_reason":"{stop_reason}","session_id":"f0e5","total_cost_usd":0.01,"usage":{{"input_tokens":2,"output_tokens":16}}}}"#
    )
}

#[test]
fn a_claude_cli_run_stopped_by_max_tokens_is_truncated() {
    let c = ClaudeCliProvider::parse_result(&cli_result("max_tokens")).unwrap();
    assert_eq!(c.text, "half a sen");
    assert!(c.truncated);
}

#[test]
fn a_claude_cli_run_that_ended_its_turn_or_says_nothing_is_not() {
    let c = ClaudeCliProvider::parse_result(&cli_result("end_turn")).unwrap();
    assert!(!c.truncated);
    let older = r#"{"type":"result","subtype":"success","is_error":false,"result":"hi","usage":{"input_tokens":2,"output_tokens":1}}"#;
    assert!(!ClaudeCliProvider::parse_result(older).unwrap().truncated);
}

use super::*;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

type Seen = Arc<Mutex<Vec<(&'static str, String)>>>;

fn recording() -> (ChunkFn, Seen) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    let on_chunk: ChunkFn = Arc::new(move |c: Chunk<'_>| {
        sink.lock().unwrap().push(match c {
            Chunk::Text(s) => ("text", s.to_string()),
            Chunk::Thought(s) => ("thought", s.to_string()),
        });
    });
    (on_chunk, seen)
}

/// Feed `lines` through the stream state exactly as `consume_sse` does,
/// including the end-of-stream tag flush.
fn run(lines: &[&str]) -> (SseState, Vec<(&'static str, String)>, bool) {
    let (on_chunk, seen) = recording();
    let started = AtomicBool::new(false);
    let mut st = SseState::default();
    for l in lines {
        apply_sse_line(l, &on_chunk, &started, &mut st);
    }
    for piece in st.tags.finish() {
        st.route(piece, &on_chunk, &started);
    }
    let seen = seen.lock().unwrap().clone();
    (st, seen, started.load(std::sync::atomic::Ordering::SeqCst))
}

#[test]
fn reasoning_under_any_of_its_three_names_is_a_thought() {
    let dash = r#"data: {"choices":[{"delta":{"reasoning_content":"hmm","content":null}}]}"#;
    assert_eq!(parse_sse_frame(dash), vec![SseItem::Thought("hmm".into())]);
    let or = r#"data: {"choices":[{"delta":{"reasoning":"so"}}]}"#;
    assert_eq!(parse_sse_frame(or), vec![SseItem::Thought("so".into())]);
    let details = r#"data: {"choices":[{"delta":{"reasoning_details":[{"type":"reasoning.text","text":"a"},{"type":"reasoning.summary","summary":"b"},{"type":"reasoning.encrypted","data":"zzz"}]}}]}"#;
    assert_eq!(
        parse_sse_frame(details),
        vec![SseItem::Thought("ab".into())]
    );
}

#[test]
fn a_frame_carrying_text_and_usage_yields_both_in_order() {
    let line = r#"data: {"choices":[{"delta":{"content":"hi"}}],"usage":{"prompt_tokens":2,"completion_tokens":1}}"#;
    assert_eq!(
        parse_sse_frame(line),
        vec![SseItem::Delta("hi".into()), SseItem::Usage(2, 1, 0)]
    );
    assert_eq!(parse_sse_frame(": keep-alive"), Vec::<SseItem>::new());
    assert_eq!(parse_sse_frame("data: [DONE]"), vec![SseItem::Done]);
    assert_eq!(parse_sse_frame("data: {not json"), Vec::<SseItem>::new());
}

#[test]
fn think_tags_split_across_frames_route_to_the_thought_and_leave_the_reply_clean() {
    let (st, seen, started) = run(&[
        r#"data: {"choices":[{"delta":{"content":"<thi"}}]}"#,
        r#"data: {"choices":[{"delta":{"content":"nk>foo</th"}}]}"#,
        r#"data: {"choices":[{"delta":{"content":"ink>bar"}}]}"#,
        "data: [DONE]",
    ]);
    assert_eq!(
        seen,
        vec![("thought", "foo".to_string()), ("text", "bar".to_string())]
    );
    assert_eq!(st.text, "bar");
    assert_eq!(st.thought, "foo");
    assert!(started, "visible text flips `started`");
}

#[test]
fn a_thought_alone_does_not_flip_started() {
    let (st, seen, started) =
        run(&[r#"data: {"choices":[{"delta":{"reasoning_content":"weighing"}}]}"#]);
    assert_eq!(seen, vec![("thought", "weighing".to_string())]);
    assert!(st.any_frame, "a thought is a real frame, not an error body");
    assert!(!started, "only reply text pins the attempt to this model");
}

#[test]
fn tool_call_fragments_across_frames_assemble_into_one_call() {
    let (st, seen, _) = run(&[
        r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"c1","type":"function","function":{"name":"sys_run","arguments":""}}]}}]}"#,
        r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"cmd\":"}}]}}]}"#,
        r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"ls\"}"}}]}}]}"#,
        "data: [DONE]",
    ]);
    assert!(seen.is_empty(), "fragments are not text");
    let calls = st.calls.finish();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id, "c1");
    assert_eq!(calls[0].name, "sys_run");
    assert_eq!(calls[0].input, serde_json::json!({"cmd": "ls"}));
}

#[test]
fn a_non_streamed_reply_carries_its_reasoning_field_and_its_think_tags_as_thought() {
    let body = r#"{"choices":[{"message":{"role":"assistant","content":"the answer","reasoning_content":"first, consider"}}],"usage":{"prompt_tokens":3,"completion_tokens":4}}"#;
    let c = parse_response(body).unwrap();
    assert_eq!(c.text, "the answer");
    assert_eq!(c.thought, "first, consider");

    let tagged = r#"{"choices":[{"message":{"content":"<think>\nweigh it\n</think>\n\nanswer"}}],"usage":{"prompt_tokens":1,"completion_tokens":1}}"#;
    let c = parse_response(tagged).unwrap();
    assert_eq!(
        c.text, "\n\nanswer",
        "the tags and the working leave the reply"
    );
    assert_eq!(c.thought, "weigh it");

    let or = r#"{"choices":[{"message":{"content":"x","reasoning":"r"}}],"usage":{"prompt_tokens":1,"completion_tokens":1}}"#;
    assert_eq!(parse_response(or).unwrap().thought, "r");
}

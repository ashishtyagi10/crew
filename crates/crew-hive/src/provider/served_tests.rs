use std::sync::Arc;

use super::super::anthropicsse::Fold;
use super::super::openai_http::{parse_response, parse_sse_frame, SseItem};
use super::super::{AnthropicProvider, Chunk, ChunkFn};
use crate::graph::ModelTier;
use crate::pricing::usage_cost;

fn quiet() -> ChunkFn {
    Arc::new(|_: Chunk<'_>| {})
}

/// DashScope's implicit cache on an OpenAI-shape body: 800 of the 1000
/// prompt tokens were cache hits. qwen-plus: 200 × $0.4 + 800 × $0.08
/// (20%) + 100 × $1.2 per M = 80 + 64 + 120 µ$ — not the 400 + 120 of
/// billing the whole prompt at the input rate.
#[test]
fn openai_cached_tokens_are_billed_at_the_cached_rate() {
    let body = r#"{"model":"qwen-plus","choices":[{"message":{"content":"hi"}}],
        "usage":{"prompt_tokens":1000,"completion_tokens":100,
        "prompt_tokens_details":{"cached_tokens":800}}}"#;
    let c = parse_response(body).unwrap();
    assert_eq!(
        c.input_tokens, 1000,
        "the count shown stays the whole prompt"
    );
    assert_eq!((c.cached_input_tokens, c.cache_write_tokens), (800, 0));
    assert_eq!(c.model, "qwen-plus");
    assert_eq!(usage_cost("qwen-plus", &c), 80 + 64 + 120);
    let billed = crate::apiagent::billed("qwen-plus", ModelTier::Standard, &c);
    assert_eq!(billed, 264);
    assert_ne!(billed, 400 + 120, "cache hits billed as full input");
}

/// The streamed usage frame carries the same details, and names its model.
#[test]
fn the_streamed_usage_frame_reports_cache_hits_and_the_model() {
    let line = r#"data: {"model":"qwen-flash","choices":[],"usage":{"prompt_tokens":1000,"completion_tokens":100,"prompt_tokens_details":{"cached_tokens":800}}}"#;
    assert_eq!(
        parse_sse_frame(line),
        vec![
            SseItem::Usage(1000, 100, 0),
            SseItem::Cached(800, 0),
            SseItem::Model("qwen-flash".into()),
        ]
    );
}

/// A body with no details at all — most hosts, most calls — reads as no
/// cache, and a null `model` as none.
#[test]
fn absent_details_read_as_no_cache() {
    let body = r#"{"model":null,"choices":[{"message":{"content":"hi"}}],
        "usage":{"prompt_tokens":10,"completion_tokens":5,"prompt_tokens_details":null}}"#;
    let c = parse_response(body).unwrap();
    assert_eq!((c.cached_input_tokens, c.cache_write_tokens), (0, 0));
    assert_eq!(c.model, "");
}

/// Anthropic's `input_tokens` is the UNCACHED remainder: 100 fresh + 900
/// read from cache is a 1000-token prompt. Sonnet: 100 × $3 + 900 × $0.3
/// (0.1×) + 10 × $15 per M = 300 + 270 + 150 µ$.
#[test]
fn anthropic_cache_reads_are_billed_at_a_tenth_of_input() {
    let body = r#"{"type":"message","model":"claude-sonnet-4-6",
        "content":[{"type":"text","text":"hi"}],"stop_reason":"end_turn",
        "usage":{"input_tokens":100,"cache_read_input_tokens":900,
        "cache_creation_input_tokens":0,"output_tokens":10}}"#;
    let c = AnthropicProvider::parse_response(body).unwrap();
    assert_eq!(c.input_tokens, 1000);
    assert_eq!((c.cached_input_tokens, c.cache_write_tokens), (900, 0));
    assert_eq!(c.model, "claude-sonnet-4-6");
    let billed = usage_cost("claude-sonnet-4-6", &c);
    assert_eq!(billed, 300 + 270 + 150);
    assert_ne!(billed, 3000 + 150, "cache reads billed as full input");
}

/// A cache WRITE costs 1.25× input: 400 written at $3.75 per M is 1500 µ$.
#[test]
fn anthropic_cache_writes_are_billed_at_one_and_a_quarter() {
    let body = r#"{"type":"message","content":[{"type":"text","text":"hi"}],
        "usage":{"input_tokens":100,"cache_creation_input_tokens":400,
        "cache_read_input_tokens":null,"output_tokens":0}}"#;
    let c = AnthropicProvider::parse_response(body).unwrap();
    assert_eq!(c.input_tokens, 500);
    assert_eq!((c.cached_input_tokens, c.cache_write_tokens), (0, 400));
    assert_eq!(usage_cost("claude-sonnet-4-6", &c), 300 + 1500);
}

/// The streamed reply reads the same from `message_start`.
#[test]
fn an_anthropic_stream_reads_cache_and_model_from_message_start() {
    let f = quiet();
    let mut fold = Fold::default();
    fold.feed(
        "data: {\"type\":\"message_start\",\"message\":{\"model\":\"claude-haiku-4-5\",\
         \"usage\":{\"input_tokens\":100,\"cache_read_input_tokens\":900,\"output_tokens\":1}}}\n\
         data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}\n",
        &f,
    );
    let c = fold.finish(&f).unwrap();
    assert_eq!((c.input_tokens, c.cached_input_tokens), (1000, 900));
    assert_eq!(c.model, "claude-haiku-4-5");
}

/// A cached tool loop's later round both READS the prefix the last round
/// wrote and WRITES the turns added since (`anthropiccache`), so one reply
/// carries both counts. Sonnet: 50 fresh × $3 + 1800 read × $0.3 + 200
/// written × $3.75 + 16 out × $15 per M = 150 + 540 + 750 + 240 µ$ — each
/// part at its own rate, whole or streamed.
#[test]
fn a_reply_that_reads_and_writes_the_cache_prices_each_part() {
    let body = r#"{"type":"message","model":"claude-sonnet-5-5",
        "content":[{"type":"text","text":"hi"}],"stop_reason":"end_turn",
        "usage":{"input_tokens":50,"cache_read_input_tokens":1800,
        "cache_creation_input_tokens":200,"output_tokens":16}}"#;
    let whole = AnthropicProvider::parse_response(body).unwrap();
    let f = quiet();
    let mut fold = Fold::default();
    fold.feed(
        "data: {\"type\":\"message_start\",\"message\":{\"model\":\"claude-sonnet-5-5\",\
         \"usage\":{\"input_tokens\":50,\"cache_read_input_tokens\":1800,\
         \"cache_creation_input_tokens\":200,\"output_tokens\":1}}}\n\
         data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":16}}\n",
        &f,
    );
    let streamed = fold.finish(&f).unwrap();
    for c in [whole, streamed] {
        assert_eq!(
            c.input_tokens, 2050,
            "the whole prompt, both cache parts in"
        );
        assert_eq!((c.cached_input_tokens, c.cache_write_tokens), (1800, 200));
        assert_eq!(c.output_tokens, 16);
        // $2/$10, a cache read $0.20, a write 1.25× input.
        assert_eq!(usage_cost("claude-sonnet-5-5", &c), 100 + 360 + 500 + 160);
    }
}

/// A reply that named no model is stamped with the attempt's; one that did
/// keeps its own word.
#[test]
fn served_by_fills_only_an_empty_model() {
    let blank = super::Completion::default();
    assert_eq!(blank.served_by("qwen-plus").model, "qwen-plus");
    let named = super::Completion {
        model: "qwen-plus-2025-12-01".into(),
        ..Default::default()
    };
    assert_eq!(named.served_by("qwen-plus").model, "qwen-plus-2025-12-01");
}

//! A stream that closes cleanly before it says it is over was CUT, not
//! finished, and fails as a broken stream does — so `crew-plugin`'s `resume`
//! picks it up where it stopped instead of it standing as the whole reply.
//! Over a socket, on both wire paths: a host or proxy that drops the
//! connection between deltas sends a clean close, not a transport error.
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::provider::anthropic::retry_tests::{serve, Reply};
use crate::provider::{
    AnthropicProvider, Chunk, ChunkFn, Completion, CompletionRequest, OpenRouterProvider, Provider,
    ProviderError,
};

/// Where the live probe's reply stopped (2026-10-22), taken as all of it.
const CUT: &str = "The fix is in clip(); the two bran";

/// A 200 event stream with no length, closed once `body` is written: the
/// shape a dropped connection leaves, end of body and all.
fn closed(body: &str) -> Reply {
    let head = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n";
    vec![head.as_bytes().to_vec(), body.as_bytes().to_vec()]
}

/// One OpenAI-shaped text delta.
fn delta(s: &str) -> String {
    format!("data: {{\"choices\":[{{\"delta\":{{\"content\":\"{s}\"}}}}]}}\n\n")
}

/// Anthropic's opening and one text delta of [`CUT`], then `end`.
fn anthropic(end: &str) -> String {
    format!(
        "event: message_start\ndata: {{\"type\":\"message_start\",\"message\":{{\"usage\":\
         {{\"input_tokens\":9}}}}}}\n\nevent: content_block_delta\ndata: {{\"type\":\
         \"content_block_delta\",\"delta\":{{\"type\":\"text_delta\",\"text\":\"{CUT}\"}}}}\n\n{end}"
    )
}

fn openai(base: &str) -> OpenRouterProvider {
    OpenRouterProvider::new("k".into()).with_endpoint(format!("{base}/v1/chat/completions"))
}

/// The result, and the text fragments the pane was handed.
async fn run(p: &dyn Provider) -> (Result<Completion, ProviderError>, Vec<String>) {
    let got = Arc::new(Mutex::new(Vec::<String>::new()));
    let g = Arc::clone(&got);
    let on_chunk: ChunkFn = Arc::new(move |c: Chunk<'_>| {
        if let Chunk::Text(s) = c {
            g.lock().unwrap().push(s.to_string());
        }
    });
    let req = CompletionRequest {
        model: "primary".into(),
        prompt: "hi".into(),
        max_tokens: 16,
        ..Default::default()
    };
    let fut = p.complete_streaming(req, on_chunk);
    let res = tokio::time::timeout(Duration::from_secs(10), fut).await;
    let chunks = got.lock().unwrap().clone();
    (res.expect("must not hang"), chunks)
}

/// A cut as the caller must see it: a failure that passes
/// (`is_transient`, which `resume` asks of it), saying what happened.
fn assert_cut(res: &Result<Completion, ProviderError>) {
    match res {
        Err(e @ ProviderError::Http(said)) => {
            assert!(
                said.contains("closed the stream before the reply ended"),
                "{said}"
            );
            assert!(
                e.is_transient(),
                "a cut stream is picked up, not given up on"
            );
        }
        other => panic!("a cut reply must not stand as the whole one: {other:?}"),
    }
}

#[tokio::test]
async fn an_openai_reply_cut_by_a_clean_close_fails_and_no_other_model_splices_on() {
    let other = closed(&format!("{}data: [DONE]\n\n", delta("another model")));
    let (base, asked) = serve(vec![closed(&delta(CUT)), other]);
    let p = openai(&base).with_fallbacks(vec!["backup".into()]);
    let (res, chunks) = run(&p).await;
    assert_cut(&res);
    assert_eq!(chunks, [CUT], "shown once");
    assert_eq!(
        asked.load(Ordering::SeqCst),
        1,
        "text was shown: the fallback model must not be asked"
    );
}

#[tokio::test]
async fn an_openai_reply_ending_in_done_is_whole() {
    let (base, _) = serve(vec![closed(&format!("{}data: [DONE]\n\n", delta(CUT)))]);
    assert_eq!(run(&openai(&base)).await.0.unwrap().text, CUT);
}

/// Some hosts close after the `finish_reason` frame and never send `[DONE]`.
#[tokio::test]
async fn an_openai_reply_ending_in_a_finish_reason_without_done_is_whole() {
    let stop = "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n";
    let (base, _) = serve(vec![closed(&format!("{}{stop}", delta(CUT)))]);
    let c = run(&openai(&base)).await.0.unwrap();
    assert_eq!(c.text, CUT);
    assert!(!c.truncated);
}

/// Half a `sys:write_file`'s arguments would otherwise run as a call whose
/// JSON is "not valid", saying nothing of why.
#[tokio::test]
async fn a_tool_call_cut_mid_arguments_fails_rather_than_standing() {
    let frag = "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"c1\",\
                \"function\":{\"name\":\"sys:write_file\",\"arguments\":\"{\\\"path\\\":\"}}]}}]}\n\n";
    let (base, _) = serve(vec![closed(frag)]);
    assert_cut(&run(&openai(&base)).await.0);
}

/// Reasoning alone, cut, stays what it was: a blank reply, which `resume`
/// asks again — nothing was shown to pick up from.
#[tokio::test]
async fn a_cut_stream_of_reasoning_alone_stays_a_blank_reply() {
    let thought = "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"hmm\"}}]}\n\n";
    let (base, _) = serve(vec![closed(thought)]);
    let c = run(&openai(&base)).await.0.unwrap();
    assert_eq!((c.text.as_str(), c.thought.as_str()), ("", "hmm"));
}

#[tokio::test]
async fn an_anthropic_reply_cut_by_a_clean_close_fails_and_is_not_typed_out_again() {
    let stop = "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
    let (base, asked) = serve(vec![closed(&anthropic("")), closed(&anthropic(stop))]);
    let p = AnthropicProvider::new("k".into()).with_base_url(&base);
    let (res, chunks) = run(&p).await;
    assert_cut(&res);
    assert_eq!(chunks, [CUT], "shown once");
    assert_eq!(
        asked.load(Ordering::SeqCst),
        1,
        "a second try would type it again"
    );
}

#[tokio::test]
async fn an_anthropic_reply_ending_in_message_stop_or_a_stop_reason_is_whole() {
    let stop = "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
    let why = "event: message_delta\ndata: {\"type\":\"message_delta\",\
               \"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":9}}\n\n";
    for end in [stop, why] {
        let (base, _) = serve(vec![closed(&anthropic(end))]);
        let p = AnthropicProvider::new("k".into()).with_base_url(&base);
        assert_eq!(run(&p).await.0.unwrap().text, CUT, "ended by {end}");
    }
}

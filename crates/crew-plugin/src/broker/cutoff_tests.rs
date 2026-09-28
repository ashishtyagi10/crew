//! Through the relay's own adapter: a scripted provider answers each call in
//! turn, and records what it was asked.
use super::*;
use crate::broker::adapter::{Adapter, HopStream};
use crate::broker::apiadapter::ApiAdapter;
use crew_hive::Chunk;
use std::collections::VecDeque;
use std::sync::Mutex;

struct Scripted {
    replies: Mutex<VecDeque<Completion>>,
    asked: Mutex<Vec<CompletionRequest>>,
    /// The call (0-based) that takes this long to answer.
    slow: Option<(usize, Duration)>,
}

impl Provider for Scripted {
    fn complete(&self, req: CompletionRequest) -> Call {
        let n = {
            let mut asked = self.asked.lock().unwrap();
            asked.push(req);
            asked.len() - 1
        };
        let c = self.replies.lock().unwrap().pop_front().unwrap_or_default();
        let wait = self.slow.filter(|(i, _)| *i == n).map(|(_, d)| d);
        Box::pin(async move {
            if let Some(d) = wait {
                tokio::time::sleep(d).await;
            }
            Ok(c)
        })
    }

    /// Each reply arrives as its words, one chunk apiece.
    fn complete_streaming(&self, req: CompletionRequest, on_chunk: ChunkFn) -> Call {
        let fut = self.complete(req);
        Box::pin(async move {
            let c = fut.await?;
            for word in c.text.split_inclusive(' ') {
                on_chunk(Chunk::Text(word));
            }
            Ok(c)
        })
    }
}

fn reply(text: &str, truncated: bool) -> Completion {
    Completion {
        text: text.into(),
        input_tokens: 10,
        output_tokens: if truncated { 8192 } else { 3 },
        truncated,
        ..Default::default()
    }
}

fn scripted(replies: Vec<Completion>, slow: Option<(usize, Duration)>) -> Arc<Scripted> {
    Arc::new(Scripted {
        replies: Mutex::new(replies.into()),
        asked: Mutex::new(Vec::new()),
        slow,
    })
}

fn adapter(p: &Arc<Scripted>) -> ApiAdapter {
    let provider: Arc<dyn Provider> = p.clone();
    ApiAdapter::new("planner", "m", "", None, provider).unwrap()
}

const SECS: Duration = Duration::from_secs(5);

#[test]
fn a_cut_reply_is_continued_once_and_the_halves_joined() {
    let p = scripted(
        vec![reply("The answer is forty", true), reply("-two.", false)],
        None,
    );
    let (text, usage) = adapter(&p).call_with_usage("task", SECS).unwrap();
    assert_eq!(text, "The answer is forty-two.");
    assert_eq!(usage.output_tokens, 8195, "both halves were billed");
    let asked = p.asked.lock().unwrap();
    assert_eq!(asked.len(), 2, "one reply, one continuation");
    assert_eq!(asked[0].max_tokens, 4096);
    assert_eq!(asked[1].prompt, asked[0].prompt);
    assert_eq!(
        asked[1].turns,
        vec![
            Turn::Assistant {
                text: "The answer is forty".into(),
                calls: Vec::new(),
            },
            Turn::User(CONTINUE.into()),
        ]
    );
}

#[test]
fn a_reply_cut_twice_stops_at_two_calls_and_says_so() {
    let p = scripted(
        vec![reply("a ", true), reply("b", true), reply("c", false)],
        None,
    );
    let text = adapter(&p).call("task", SECS).unwrap();
    assert_eq!(p.asked.lock().unwrap().len(), 2, "never a third call");
    assert_eq!(text, format!("a b\n\n{CUT_OFF}"));
    assert!(text.ends_with("\u{2026} (cut off at the token limit)"));
}

#[test]
fn a_finished_reply_is_one_call_and_unmarked() {
    let p = scripted(vec![reply("whole", false), reply("never", false)], None);
    assert_eq!(adapter(&p).call("task", SECS).unwrap(), "whole");
    assert_eq!(p.asked.lock().unwrap().len(), 1);
}

#[test]
fn a_reply_cut_before_it_said_anything_is_marked_not_continued() {
    let p = scripted(vec![reply("  ", true), reply("never", false)], None);
    assert_eq!(adapter(&p).call("task", SECS).unwrap(), CUT_OFF);
    assert_eq!(p.asked.lock().unwrap().len(), 1);
}

#[test]
fn the_continuation_streams_on_from_where_the_reply_stopped() {
    let p = scripted(
        vec![reply("The answer is forty", true), reply("-two.", false)],
        None,
    );
    let seen = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&seen);
    let stream = HopStream {
        on_text: Arc::new(move |s| sink.lock().unwrap().push_str(s)),
        ..HopStream::noop()
    };
    let (text, _) = adapter(&p)
        .call_with_usage_ticked("task", SECS, &stream)
        .unwrap();
    assert_eq!(text, "The answer is forty-two.");
    assert_eq!(*seen.lock().unwrap(), "The answer is forty-two.");
    assert_eq!(p.asked.lock().unwrap().len(), 2);
}

/// The continuation stalls past the caller's budget: the first half comes
/// back, marked, instead of the whole hop failing as timed out.
#[test]
fn a_continuation_that_outlives_the_budget_keeps_the_first_half() {
    let p = scripted(
        vec![reply("The answer is forty", true), reply("-two.", false)],
        Some((1, Duration::from_secs(30))),
    );
    let text = adapter(&p)
        .call("task", Duration::from_secs(2))
        .expect("the first half, not a timeout");
    assert_eq!(text, format!("The answer is forty\n\n{CUT_OFF}"));
}

#[test]
fn marked_adds_the_line_only_to_a_cut_reply() {
    assert_eq!(marked("done.", false), "done.");
    assert_eq!(
        marked("half a sen  \n", true),
        format!("half a sen\n\n{CUT_OFF}")
    );
    assert_eq!(marked("", true), CUT_OFF);
}

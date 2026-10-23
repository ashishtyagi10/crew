//! Through the relay's own adapter, as `cutoff_tests` does: a flaky provider
//! plays each call's part in turn — streams its words, then answers with
//! them or fails — and records what it was asked.
use super::super::Call;
use super::*;
use crate::broker::adapter::{Adapter, HopStream};
use crate::broker::apiadapter::ApiAdapter;
use crew_hive::Turn;
use std::collections::VecDeque;

/// One call's part: the words it streams, then the failure it ends on.
type Part = (&'static str, Option<ProviderError>);

struct Flaky {
    parts: Mutex<VecDeque<Part>>,
    asked: Mutex<Vec<CompletionRequest>>,
    /// The call (0-based) that stalls for half a minute before its part.
    slow: Option<usize>,
}

impl Provider for Flaky {
    fn complete(&self, req: CompletionRequest) -> Call {
        self.complete_streaming(req, Arc::new(|_| {}))
    }

    /// Each word arrives as its own chunk, and only then does the call end.
    fn complete_streaming(&self, req: CompletionRequest, on_chunk: ChunkFn) -> Call {
        let n = {
            let mut asked = self.asked.lock().unwrap();
            asked.push(req);
            asked.len() - 1
        };
        let (said, ends) = self.parts.lock().unwrap().pop_front().unwrap_or(("", None));
        let stall = self.slow == Some(n);
        Box::pin(async move {
            if stall {
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
            for word in said.split_inclusive(' ') {
                on_chunk(Chunk::Text(word));
            }
            match ends {
                Some(e) => Err(e),
                None => Ok(Completion {
                    text: said.into(),
                    output_tokens: 3,
                    ..Default::default()
                }),
            }
        })
    }
}

fn flaky(parts: Vec<Part>, slow: Option<usize>) -> Arc<Flaky> {
    Arc::new(Flaky {
        parts: Mutex::new(parts.into()),
        asked: Mutex::new(Vec::new()),
        slow,
    })
}

fn adapter(p: &Arc<Flaky>) -> ApiAdapter {
    let provider: Arc<dyn Provider> = p.clone();
    ApiAdapter::new("planner", "m", "", None, provider).unwrap()
}

/// The wire giving out mid-reply, as `wire::wire_error` says it.
fn dropped() -> Option<ProviderError> {
    Some(ProviderError::Http(
        "api.example.com dropped the response mid-stream (connection reset by peer)".into(),
    ))
}

/// The reply streamed within `budget`, and every word the pane was shown.
fn streamed(p: &Arc<Flaky>, budget: Duration) -> (Result<String, String>, String) {
    let seen = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&seen);
    let stream = HopStream {
        on_text: Arc::new(move |s| sink.lock().unwrap().push_str(s)),
        ..HopStream::noop()
    };
    let text = adapter(p)
        .call_with_usage_ticked("task", budget, &stream)
        .map(|(t, _)| t);
    let seen = seen.lock().unwrap().clone();
    (text, seen)
}

const SECS: Duration = Duration::from_secs(5);

#[test]
fn a_stream_that_broke_mid_reply_is_picked_up_where_it_stopped() {
    let p = flaky(vec![("Hello wor", dropped()), ("ld.", None)], None);
    let (text, seen) = streamed(&p, SECS);
    assert_eq!(text.unwrap(), "Hello world.");
    assert_eq!(
        seen, "Hello world.",
        "each word shown once, across the seam"
    );
    let asked = p.asked.lock().unwrap();
    assert_eq!(asked.len(), 2, "one reply, one pick-up");
    assert_eq!(asked[1].prompt, asked[0].prompt);
    assert_eq!(
        asked[1].turns,
        vec![
            Turn::Assistant {
                text: "Hello wor".into(),
                calls: Vec::new(),
            },
            Turn::User(resume_line("Hello wor")),
        ]
    );
}

#[test]
fn an_empty_reply_is_asked_again_until_one_answers() {
    let p = flaky(vec![("", None), ("  ", None), ("Here.", None)], None);
    assert_eq!(adapter(&p).call("task", SECS).unwrap(), "Here.");
    let asked = p.asked.lock().unwrap();
    assert_eq!(asked.len(), 3);
    assert!(
        asked
            .iter()
            .all(|r| r.turns.is_empty() && r.prompt == asked[0].prompt),
        "the same request each time"
    );
}

#[test]
fn three_empty_replies_are_the_end_of_it() {
    let p = flaky(
        vec![("", None), ("", None), ("", None), ("never", None)],
        None,
    );
    assert_eq!(adapter(&p).call("task", SECS).unwrap(), "");
    assert_eq!(p.asked.lock().unwrap().len(), 3, "never a fourth call");
}

#[test]
fn a_missing_or_rejected_key_is_never_asked_again() {
    for e in [
        ProviderError::MissingKey("X_API_KEY"),
        ProviderError::Status("HTTP 401 from api.example.com: Unauthorized".into()),
    ] {
        let p = flaky(vec![("Hello wor", Some(e)), ("ld.", None)], None);
        assert!(streamed(&p, SECS).0.is_err());
        assert_eq!(p.asked.lock().unwrap().len(), 1);
    }
}

#[test]
fn a_failure_before_the_first_word_is_left_to_the_provider() {
    let p = flaky(vec![("", dropped()), ("never", None)], None);
    let (text, seen) = streamed(&p, SECS);
    assert!(text.unwrap_err().contains("dropped the response"));
    assert_eq!(seen, "");
    assert_eq!(p.asked.lock().unwrap().len(), 1);
}

/// The pick-up stalls past the caller's budget: the hop fails as the stream
/// did, not as timed out.
#[test]
fn a_pick_up_that_outlives_the_budget_leaves_the_stream_failed() {
    let p = flaky(vec![("Hello wor", dropped()), ("ld.", None)], Some(1));
    let (text, seen) = streamed(&p, Duration::from_secs(2));
    assert!(text.unwrap_err().contains("dropped the response"));
    assert_eq!(seen, "Hello wor");
    assert_eq!(p.asked.lock().unwrap().len(), 2);
}

/// The line quotes where the reply stopped, at most its last forty
/// characters, cut on a character boundary.
#[test]
fn the_resume_line_quotes_where_the_reply_stopped() {
    assert!(resume_line("Hello wor").contains("\u{ab}Hello wor\u{bb}"));
    let long = "\u{e9}".repeat(50);
    let line = resume_line(&long);
    assert!(
        line.contains(&format!("\u{ab}{}\u{bb}", "\u{e9}".repeat(40))),
        "{line}"
    );
}

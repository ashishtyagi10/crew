use super::kit::*;
use super::*;
use crate::agent::Agent;
use crate::bus::EventBus;
use crate::provider::{Completion, ProviderError};
use crate::tools::MAX_TOOL_ROUNDS;
use std::sync::Mutex;

#[tokio::test]
async fn a_structured_call_runs_and_its_result_goes_back_as_a_paired_turn() {
    let (out, calls, seen) = run_agent(
        vec![
            calling("call_1", "weather__current"),
            answering("4C in Oslo"),
        ],
        Ok("Oslo 4C clear".into()),
    )
    .await;

    assert!(out.success);
    assert_eq!(out.output, "4C in Oslo");
    assert_eq!(
        &*calls.lock().unwrap(),
        &["weather:current {\"q\":\"Oslo\"}"],
        "the wire name must resolve back to crew's server:tool"
    );

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    // The model was shown the SCHEMA, not a text hint.
    assert_eq!(seen[0].tools.len(), 1);
    assert_eq!(seen[0].tools[0].name, "weather__current");
    assert!(
        !seen[0].prompt.contains("@tool"),
        "the text convention must not be advertised alongside: {}",
        seen[0].prompt
    );
    // The follow-up replays the call and pairs the result by id.
    assert_eq!(seen[1].turns.len(), 2);
    match (&seen[1].turns[0], &seen[1].turns[1]) {
        (Turn::Assistant { calls, .. }, Turn::ToolResults(results)) => {
            assert_eq!(calls[0].id, "call_1");
            assert_eq!(results[0].id, "call_1", "unpaired ids are a protocol error");
            assert_eq!(results[0].content, "Oslo 4C clear");
            assert!(!results[0].is_error);
        }
        other => panic!("wrong turn shape: {other:?}"),
    }
}

#[tokio::test]
async fn a_failed_tool_comes_back_flagged_as_an_error() {
    let (out, _, seen) = run_agent(
        vec![
            calling("c", "weather__current"),
            answering("could not check"),
        ],
        Err("connection refused".into()),
    )
    .await;

    assert!(out.success, "a dead tool is not a failed task");
    let seen = seen.lock().unwrap();
    match &seen[1].turns[1] {
        Turn::ToolResults(r) => {
            assert!(r[0].is_error, "without the flag a model reads this as data");
            assert!(r[0].content.contains("connection refused"));
        }
        other => panic!("{other:?}"),
    }
}
#[tokio::test]
async fn the_round_cap_stops_the_loop_and_says_so_in_the_answer() {
    let replies: Vec<Completion> = (0..10)
        .map(|i| calling(&format!("c{i}"), "weather__current"))
        .collect();
    let (out, calls, _) = run_agent(replies, Ok("4C".into())).await;

    // The rounds the budget gave, and the batch it stopped: that one still
    // runs, for the last word to read.
    assert_eq!(calls.lock().unwrap().len(), MAX_TOOL_ROUNDS as usize + 1);
    assert!(out.output.contains("tool budget spent"), "{}", out.output);
}

#[tokio::test]
async fn a_provider_without_tool_support_falls_back_to_the_text_convention() {
    /// Same scripted replies, but `supports_tools()` is false.
    struct NoTools(Arc<Mutex<Vec<CompletionRequest>>>);
    impl Provider for NoTools {
        fn complete(
            &self,
            req: CompletionRequest,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
        > {
            self.0.lock().unwrap().push(req);
            Box::pin(async move { Ok(answering("plain answer")) })
        }
    }
    let seen = Arc::new(Mutex::new(Vec::new()));
    let calls = Arc::new(Mutex::new(Vec::new()));
    let bus = EventBus::new(32);
    let out = crate::apiagent::ApiAgent::new(Arc::new(NoTools(Arc::clone(&seen))), 256)
        .with_tools(Arc::new(FakeTools {
            calls,
            result: Ok("x".into()),
        }))
        .run(ctx(&bus))
        .await;

    assert_eq!(out.output, "plain answer");
    let seen = seen.lock().unwrap();
    // No tools on the wire — and the TEXT hint is present instead, so the
    // agent is not left with no way to call anything at all.
    assert!(seen[0].tools.is_empty());
    assert!(seen[0].prompt.contains("unused on the native path"));
}

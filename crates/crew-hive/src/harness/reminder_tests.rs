//! The low-budget line: when it appears, where it goes, and what it leaves
//! alone. Driven through the scripted native loop with a 10-round pool.
use super::kit;
use crate::agent::{Agent, AgentContext};
use crate::bus::{EventBus, HiveEvent};
use crate::harness::reminder::line;
use crate::provider::{CompletionRequest, Turn};
use crate::tools::budget::ToolBudget;

const NEEDLE: &str = "tool round";

/// Results text per request, one `Vec<String>` per ToolResults turn, plus
/// every event the host saw.
async fn drive(total: u32) -> (Vec<CompletionRequest>, String) {
    let mut replies: Vec<_> = (0..total)
        .map(|i| kit::calling(&format!("c{i}"), "weather__current"))
        .collect();
    replies.push(kit::answering("done"));
    let (provider, seen) = kit::Scripted::new(replies);
    let tools = std::sync::Arc::new(kit::FakeTools {
        calls: Default::default(),
        result: Ok("4C".into()),
    });
    let bus = EventBus::new(256);
    let mut rx = bus.subscribe();
    let ctx = AgentContext {
        budget: ToolBudget::turn(total),
        ..kit::ctx(&bus)
    };
    crate::apiagent::ApiAgent::new(provider, 256)
        .with_tools(tools)
        .run(ctx)
        .await;
    let mut events = String::new();
    while let Ok(e) = rx.try_recv() {
        if matches!(e, HiveEvent::ToolResult { .. } | HiveEvent::ToolCall { .. }) {
            events.push_str(&format!("{e:?}\n"));
        }
    }
    let reqs = seen.lock().unwrap().clone();
    (reqs, events)
}

fn last_results(req: &CompletionRequest) -> Option<Vec<String>> {
    req.turns.iter().rev().find_map(|t| match t {
        Turn::ToolResults(r) => Some(r.iter().map(|o| o.content.clone()).collect()),
        _ => None,
    })
}

#[tokio::test]
async fn ten_rounds_warn_on_exactly_the_two_before_the_last() {
    let (reqs, _) = drive(10).await;
    let warned: Vec<bool> = reqs[1..10]
        .iter()
        .map(|r| last_results(r).unwrap().iter().any(|c| c.contains(NEEDLE)))
        .collect();
    // Request k+1 carries round k's results; left after round k is 10 - k.
    let expect: Vec<bool> = (1..=9).map(|k| matches!(10 - k, 1 | 2)).collect();
    assert_eq!(warned, expect);
    let r8 = last_results(&reqs[8]).unwrap();
    assert!(r8[0].ends_with("[2 tool rounds left \u{2014} finish and verify the change, or say plainly what is left undone.]"));
    assert!(last_results(&reqs[9]).unwrap()[0].contains("[1 tool round left "));
}

#[tokio::test]
async fn a_budget_of_three_never_warns() {
    let (reqs, _) = drive(3).await;
    for r in &reqs[1..] {
        assert!(!last_results(r).unwrap().iter().any(|c| c.contains(NEEDLE)));
    }
}

#[tokio::test]
async fn the_host_never_sees_the_line() {
    let (_, events) = drive(10).await;
    assert!(events.contains("ToolResult"), "{events}");
    assert!(!events.contains(NEEDLE), "{events}");
}

#[tokio::test]
async fn earlier_turns_are_byte_identical_from_one_request_to_the_next() {
    let (reqs, _) = drive(10).await;
    for w in reqs.windows(2) {
        let (a, b) = (format!("{:?}", w[0].turns), format!("{:?}", w[1].turns));
        let a = a.strip_suffix(']').unwrap();
        assert!(b.starts_with(a), "prefix rewritten:\n{a}\n{b}");
    }
}

#[test]
fn line_wording_and_thresholds() {
    assert_eq!(
        line(1, 10).unwrap().split(" left").next().unwrap(),
        "[1 tool round"
    );
    assert!(line(2, 10).is_some() && line(3, 10).is_none() && line(0, 10).is_none());
    // A big budget warns over its last fifth: 100 -> 20 rounds.
    assert!(line(20, 100).is_some() && line(21, 100).is_none());
    assert!(line(1, 3).is_none() && line(1, 4).is_some() && line(2, 4).is_none());
}

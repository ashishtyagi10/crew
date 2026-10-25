//! Tool rounds: a call runs, its result reaches the next prompt, and the
//! rounds are bounded and billed.
use super::tool_kit::*;
use super::*;
use crate::agent::Agent;
use crate::bus::{EventBus, HiveEvent};
use crate::tools::MAX_TOOL_ROUNDS;

#[tokio::test]
async fn tool_call_runs_and_its_result_reaches_the_next_prompt() {
    let (provider, seen) = Scripted::new(&[
        "checking\n@tool weather:current {\"q\":\"Oslo\"}",
        "It is 4°C in Oslo.",
    ]);
    let (tools, calls) = fake(Ok("Oslo: 4C, clear".into()));
    let bus = EventBus::new(64);
    let agent = ApiAgent::new(provider, 256).with_tools(tools);

    let result = agent.run(ctx(&bus)).await;

    assert!(result.success);
    assert_eq!(result.output, "It is 4°C in Oslo.");
    // The tool actually ran, with the arguments the model wrote.
    assert_eq!(
        &*calls.lock().unwrap(),
        &["weather:current {\"q\":\"Oslo\"}"]
    );
    // Two provider calls: the ask, then the answer.
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    // The first prompt advertised the tools.
    assert!(seen[0].contains("TOOLS: @tool weather:current"));
    // The second CARRIED THE RESULT — the whole point of the loop.
    assert!(
        seen[1].contains("Oslo: 4C, clear"),
        "follow-up: {}",
        seen[1]
    );
    assert!(seen[1].contains("CALLED weather:current"));
    // …under what the agent wrote with the call, so it knows why it asked.
    assert!(
        seen[1].contains("YOUR MESSAGE:\nchecking\nCALLED weather:current"),
        "follow-up: {}",
        seen[1]
    );
}

#[tokio::test]
async fn tool_call_and_result_are_published_as_events() {
    let (provider, _) = Scripted::new(&["@tool weather:current {}", "done"]);
    let (tools, _) = fake(Ok("4C".into()));
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();
    ApiAgent::new(provider, 256)
        .with_tools(tools)
        .run(ctx(&bus))
        .await;

    let mut call = None;
    let mut res = None;
    while let Ok(ev) = rx.try_recv() {
        match ev {
            HiveEvent::ToolCall { label, args, .. } => call = Some((label, args)),
            HiveEvent::ToolResult {
                label, ok, text, ..
            } => res = Some((label, ok, text)),
            _ => {}
        }
    }
    assert_eq!(
        call,
        Some(("weather:current".to_string(), "{}".to_string()))
    );
    assert_eq!(
        res,
        Some(("weather:current".to_string(), true, "4C".to_string()))
    );
}

#[tokio::test]
async fn a_failing_tool_is_reported_to_the_agent_not_raised_as_a_task_failure() {
    let (provider, seen) = Scripted::new(&[
        "@tool weather:current {}",
        "I could not reach the weather service.",
    ]);
    let (tools, _) = fake(Err("connection refused".into()));
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();

    let result = ApiAgent::new(provider, 256)
        .with_tools(tools)
        .run(ctx(&bus))
        .await;

    // The TASK succeeded: the agent got to decide what a dead tool means.
    assert!(result.success);
    assert!(seen.lock().unwrap()[1].contains("ERROR: connection refused"));
    let mut failed = false;
    let mut result_ok = true;
    while let Ok(ev) = rx.try_recv() {
        match ev {
            HiveEvent::Failed { .. } => failed = true,
            HiveEvent::ToolResult { ok, .. } => result_ok = ok,
            _ => {}
        }
    }
    assert!(!failed, "a refused tool must not fail the agent");
    assert!(!result_ok, "the ToolResult must say it failed");
}

#[tokio::test]
async fn the_round_cap_bounds_the_calls_and_the_unrun_directive_is_stripped() {
    // Ten asks in a row: MAX_TOOL_ROUNDS rounds, and the batch the budget
    // stopped, which still runs for the last word to read.
    let asking = "@tool weather:current {}";
    let (provider, _) = Scripted::new(&[asking; 10]);
    let (tools, calls) = fake(Ok("4C".into()));
    let bus = EventBus::new(256);
    let agent = ApiAgent::new(provider, 256).with_tools(tools);

    let result = agent.run(ctx(&bus)).await;

    assert_eq!(calls.lock().unwrap().len(), MAX_TOOL_ROUNDS as usize + 1);
    // The output must not end on a directive that never ran.
    assert!(
        !result.output.contains("@tool"),
        "output: {}",
        result.output
    );
    assert!(result.output.contains("tool budget spent"));
}

#[tokio::test]
async fn without_tools_the_agent_makes_exactly_one_call_with_an_unchanged_prompt() {
    // The regression guard for every keyless and mock run.
    let (provider, seen) = Scripted::new(&["@tool weather:current {}"]);
    let bus = EventBus::new(32);
    let result = ApiAgent::new(provider, 256).run(ctx(&bus)).await;

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1, "no tools attached must mean no extra calls");
    assert_eq!(seen[0], "summarize", "the prompt must be untouched");
    // A directive with nothing to run it is just text, returned as the answer.
    assert_eq!(result.output, "@tool weather:current {}");
}

#[tokio::test]
async fn every_round_is_billed() {
    let (provider, _) = Scripted::new(&["@tool weather:current {}", "done"]);
    let (tools, _) = fake(Ok("4C".into()));
    let bus = EventBus::new(64);
    let mut rx = bus.subscribe();
    ApiAgent::new(provider, 256)
        .with_tools(tools)
        .run(ctx(&bus))
        .await;

    let mut token_events = 0;
    while let Ok(ev) = rx.try_recv() {
        if matches!(ev, HiveEvent::TokenDelta { .. }) {
            token_events += 1;
        }
    }
    // Two model calls means two token deltas, or the budget governor
    // undercounts a tool-using run by every round after the first.
    assert_eq!(token_events, 2);
}

/// The cap is the RUN's, not the task's: in a three-task run an agent that
/// needs eight rounds gets them, and the ninth is the last: it runs, and the
/// last word reads it.
#[tokio::test]
async fn a_run_budget_lets_one_agent_past_the_old_cap() {
    let asking = "@tool weather:current {}";
    let (provider, _) = Scripted::new(&[asking; 12]);
    let (tools, calls) = fake(Ok("4C".into()));
    let bus = EventBus::new(256);
    let agent = ApiAgent::new(provider, 256).with_tools(tools);
    let budget = crate::tools::budget::ToolBudget::for_run(3);
    let mut c = ctx(&bus);
    c.budget = budget.clone();
    let result = agent.run(c).await;
    assert_eq!(
        calls.lock().unwrap().len(),
        2 * MAX_TOOL_ROUNDS as usize + 1
    );
    assert!(
        result.output.contains("12 rounds of tool calls"),
        "{}",
        result.output
    );
    assert_eq!(
        budget.left(),
        4,
        "what this agent did not take is still in the pool"
    );
}

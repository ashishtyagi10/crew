//! One decision is one call — and the scorer's set, exactly, whenever the model made none.
//! Every counting test holds `testenv`'s lock: `CREW_TOOL_PICK=0` is process-wide, and the
//! test that sets it must not be able to zero a neighbour's count.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::*;
use crate::broker::testenv;

fn tool(server: &str, name: &str) -> McpTool {
    McpTool {
        server: server.into(),
        name: name.into(),
        description: "does an unrelated thing".into(),
        input_schema: serde_json::json!({"type": "object"}),
    }
}

/// `sys` tools first, then `n` nobody asked for.
fn catalog(n: usize) -> Vec<McpTool> {
    let mut c = vec![tool("sys", "run"), tool("sys", "find_tools")];
    c.extend((0..n).map(|i| tool("noise", &format!("thing{i}"))));
    c
}

fn labels(tools: &[McpTool]) -> Vec<String> {
    tools.iter().map(label).collect()
}

/// A chooser that answers `reply` and counts how often it was asked.
fn counting(reply: &str, calls: &Arc<AtomicUsize>) -> Picker {
    let (reply, calls) = (reply.to_string(), Arc::clone(calls));
    Picker::fixed(Box::new(move |_: &str| {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok(reply.clone())
    }))
}

#[test]
fn an_injected_chooser_decides_the_set_and_sys_and_the_door_ride_along() {
    let _env = testenv::mock("unused");
    let calls = Arc::new(AtomicUsize::new(0));
    let picker = counting("TOOLS: noise:thing30, noise:thing2", &calls);
    let (kept, left_out) = picker.pick(catalog(BUDGET + 8), "x");
    assert_eq!(
        labels(&kept),
        ["sys:run", "sys:find_tools", "noise:thing2", "noise:thing30"]
    );
    assert_eq!(left_out, BUDGET + 8 - 2);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    // With no `sys` in the catalog the door is still appended.
    let picker = counting("TOOLS: noise:thing2", &calls);
    let noise: Vec<McpTool> = catalog(BUDGET + 8).into_iter().skip(2).collect();
    let (kept, _) = picker.pick(noise, "x");
    assert_eq!(labels(&kept), ["noise:thing2", "sys:find_tools"]);
    let chosen = picker.take_chosen().expect("recorded for the pane");
    assert_eq!(chosen.of, BUDGET + 8);
    assert_eq!(chosen.names, ["noise:thing2", "sys:find_tools"]);
    assert_eq!(picker.take_chosen(), None, "taken once");
}

#[test]
fn a_chooser_error_falls_back_to_the_scorers_set_exactly() {
    let _env = testenv::mock("unused");
    let picker = Picker::fixed(Box::new(|_: &str| Err("timed out".into())));
    let task = "please read thing3 and thing5";
    let (kept, left) = picker.pick(catalog(BUDGET + 8), task);
    let (scored, scored_left) = super::super::toolselect::select(catalog(BUDGET + 8), task);
    assert_eq!(labels(&kept), labels(&scored));
    assert_eq!(left, scored_left);
    assert_eq!(
        picker.take_chosen(),
        None,
        "nothing was chosen, so nothing is said"
    );
}

#[test]
fn under_the_budget_no_call_is_made() {
    let _env = testenv::mock("unused");
    let calls = Arc::new(AtomicUsize::new(0));
    let picker = counting("TOOLS: noise:thing1", &calls);
    let (kept, left) = picker.pick(catalog(BUDGET - 2), "x");
    assert_eq!(kept.len(), BUDGET);
    assert_eq!(left, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn the_memo_makes_one_call_for_three_reads_and_a_second_for_another_task() {
    let _env = testenv::mock("unused");
    let calls = Arc::new(AtomicUsize::new(0));
    let picker = counting("TOOLS: noise:thing1", &calls);
    let first = picker.pick(catalog(BUDGET + 8), "read thing1");
    assert_eq!(picker.pick(catalog(BUDGET + 8), "read thing1"), first);
    assert_eq!(picker.pick(catalog(BUDGET + 8), "read thing1"), first);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "hint, specs and note: one decision"
    );
    picker.pick(catalog(BUDGET + 8), "something else");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "a different task is a new question"
    );
    // The same task on a catalog that grew (a server connected) is a new question too.
    picker.pick(catalog(BUDGET + 9), "read thing1");
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[test]
fn the_memo_is_bounded() {
    let _env = testenv::mock("unused");
    let calls = Arc::new(AtomicUsize::new(0));
    let picker = counting("TOOLS: noise:thing1", &calls);
    for i in 0..MEMO_CAP + 1 {
        picker.pick(catalog(BUDGET + 8), &format!("task {i}"));
    }
    assert_eq!(picker.memo.len(), MEMO_CAP);
    picker.pick(catalog(BUDGET + 8), "task 0");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        MEMO_CAP + 2,
        "the oldest was evicted"
    );
}

#[test]
fn crew_tool_pick_0_makes_no_call_and_the_scorer_decides() {
    let _env = testenv::mock("unused");
    struct Unset;
    impl Drop for Unset {
        fn drop(&mut self) {
            std::env::remove_var("CREW_TOOL_PICK");
        }
    }
    let _unset = Unset;
    std::env::set_var("CREW_TOOL_PICK", "0");
    let calls = Arc::new(AtomicUsize::new(0));
    let picker = counting("TOOLS: noise:thing1", &calls);
    let (kept, _) = picker.pick(catalog(BUDGET + 8), "read thing3");
    let (scored, _) = super::super::toolselect::select(catalog(BUDGET + 8), "read thing3");
    assert_eq!(labels(&kept), labels(&scored));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(live().is_none(), "and the live chooser is off too");
}

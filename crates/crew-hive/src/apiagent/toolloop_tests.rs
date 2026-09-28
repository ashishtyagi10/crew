use super::*;

/// A log of `(label, result)` exchanges with no message, as the loop keeps it.
fn log(rounds: &[(&str, &str)]) -> Exchanges {
    let mut log = Exchanges::default();
    for (label, result) in rounds {
        log.push(exchange("", label, "{}", result));
    }
    log
}

#[test]
fn clip_keeps_the_head_and_marks_the_cut() {
    let s = "abcdefghij";
    assert_eq!(clip(s, 20), "abcdefghij");
    assert_eq!(clip(s, 4), "abcd… [clipped 6 chars]");
}

#[test]
fn clip_counts_chars_not_bytes() {
    // Four 3-byte chars: a byte-based clip at 2 would split one and panic.
    let s = "日本語だ";
    assert_eq!(clip(s, 2), "日本… [clipped 2 chars]");
}

#[test]
fn exchange_caps_a_huge_result() {
    let huge = "x".repeat(RESULT_CAP + 500);
    let e = exchange("", "fs:read", "{}", &huge).to_string();
    assert!(e.starts_with("CALLED fs:read {}\nRESULT:\n"));
    assert!(e.contains("[clipped 500 chars]"));
    // The whole exchange must stay near the cap, not near the input size.
    assert!(e.chars().count() < RESULT_CAP + 100);
}

#[test]
fn follow_up_restates_the_base_prompt_and_every_exchange() {
    let p = follow_up(
        "TASK\n\nTOOLS: @tool sys:run",
        "",
        &log(&[("a:b", "first"), ("c:d", "second")]),
        2,
    );
    // The tools hint must survive into the follow-up, or a second call is
    // unspellable.
    assert!(p.contains("TOOLS: @tool sys:run"));
    assert!(p.contains("first"));
    assert!(p.contains("second"));
    assert!(p.contains("You may make 2 more tool call(s)"));
}

#[test]
fn follow_up_tells_the_agent_when_it_is_out_of_calls() {
    let p = follow_up("TASK", "", &log(&[("a:b", "r")]), 0);
    assert!(p.contains("LAST tool call"));
    assert!(!p.contains("more tool call(s)"));
}

#[test]
fn budget_spent_strips_the_unrun_directive() {
    let reply =
        "I checked the first city.\nNow the second:\n@tool weather:current {\"q\":\"Oslo\"}";
    let out = budget_spent(reply, 4);
    // The phantom call must not survive into the task's output, where it
    // becomes a dependency's context and gets imitated downstream.
    assert!(!out.contains("@tool"));
    assert!(out.contains("I checked the first city."));
    assert!(out.contains("[tool budget spent — 4 calls"));
}

#[test]
fn budget_spent_on_a_bare_directive_is_just_the_note() {
    let out = budget_spent("@tool weather:current {}", 4);
    assert_eq!(
        out,
        "[tool budget spent — 4 calls for this run; the last request was not run]"
    );
}

#[test]
fn budget_spent_ignores_trailing_blank_lines_when_finding_the_directive() {
    let out = budget_spent("kept text\n@tool a:b {}\n\n  \n", 2);
    assert!(!out.contains("@tool"));
    assert!(out.starts_with("kept text"));
}

/// What the agent wrote with a call sits above the call in the next prompt,
/// or the agent reads its own earlier calls without the reason for any of
/// them and makes one again.
#[test]
fn exchange_carries_what_the_agent_wrote_with_the_call() {
    let e = exchange("The bug is in clip().", "sys:read_file", "{}", "fn clip");
    assert_eq!(
        e.to_string(),
        "YOUR MESSAGE:\nThe bug is in clip().\nCALLED sys:read_file {}\nRESULT:\nfn clip"
    );
}

#[test]
fn budget_spent_strips_a_fenced_multi_line_call_whole() {
    let reply = "Two of three cities checked.\n```json\n@tool weather:current {\n  \
                 \"q\": \"Oslo\"\n}\n```\n@done";
    assert_eq!(
        budget_spent(reply, 4),
        "Two of three cities checked.\n\n\
         [tool budget spent — 4 calls for this run; the last request was not run]"
    );
}

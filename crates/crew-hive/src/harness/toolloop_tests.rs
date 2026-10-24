use super::*;

/// A log of `(label, result)` exchanges with no message, as the loop keeps it.
fn log(rounds: &[(&str, &str)]) -> Exchanges {
    let mut log = Exchanges::default();
    for (label, result) in rounds {
        log.push(exchange("", label, "{}", result));
    }
    log
}

/// A long result keeps its head, what it is about, and its end, where a
/// build says what failed and `sys:read_file` says where to go on.
#[test]
fn clip_keeps_both_ends_and_marks_the_cut() {
    let s = "abcdefghij";
    assert_eq!(clip(s, 20), "abcdefghij");
    let long = format!(
        "HEAD\n{}\nerror[E0308]: mismatched types",
        "x\n".repeat(4_000)
    );
    let cut = clip(&long, RESULT_CAP);
    assert!(cut.starts_with("HEAD\n"), "{cut}");
    assert!(cut.ends_with("error[E0308]: mismatched types"), "{cut}");
    assert!(cut.contains("chars cut from the middle"), "{cut}");
}

#[test]
fn clip_counts_chars_not_bytes() {
    // 3-byte chars: a byte-based clip would split one and panic.
    let s = "日本語だ".repeat(50);
    let cut = clip(&s, 30);
    assert!(cut.starts_with('日') && cut.ends_with('だ'), "{cut}");
}

#[test]
fn exchange_caps_a_huge_result() {
    let huge = "x".repeat(RESULT_CAP + 500);
    let e = exchange("", "fs:read", "{}", &huge).to_string();
    assert!(e.starts_with("CALLED fs:read {}\nRESULT:\n"));
    assert!(e.contains("[500 chars cut from the middle]"), "{e}");
    // The whole exchange must stay near the cap, not near the input size.
    assert!(e.chars().count() < RESULT_CAP + 100);
}

#[test]
fn follow_up_restates_the_base_prompt_and_every_exchange() {
    let p = follow_up(
        "TASK\n\nTOOLS: @tool sys:run",
        "",
        &log(&[("a:b", "first"), ("c:d", "second")]),
        "",
        2,
        "",
    );
    // The tools hint must survive into the follow-up, or a second call is
    // unspellable.
    assert!(p.contains("TOOLS: @tool sys:run"));
    assert!(p.contains("first"));
    assert!(p.contains("second"));
    assert!(
        p.contains("You may make 2 more round(s) of tool calls for this task, up to 8 calls"),
        "{p}"
    );
}

#[test]
fn follow_up_tells_the_agent_when_it_is_out_of_calls() {
    let p = follow_up("TASK", "", &log(&[("a:b", "r")]), "", 0, "");
    assert!(p.contains("LAST round of tool calls"), "{p}");
    assert!(!p.contains("more round(s)"), "{p}");
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
    assert!(out.contains("[tool budget spent — 4 rounds"), "{out}");
}

#[test]
fn budget_spent_on_a_bare_directive_is_just_the_note() {
    let out = budget_spent("@tool weather:current {}", 4);
    assert_eq!(out, "[tool budget spent — 4 rounds of tool calls]");
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
        "Two of three cities checked.\n\n[tool budget spent — 4 rounds of tool calls]"
    );
}

#[test]
fn what_the_user_added_comes_after_the_exchanges_and_before_the_budget_line() {
    let heard = crate::steers::section(&["also check the tests".into()], "TAIL");
    let p = follow_up("TASK", "", &log(&[("a:b", "r")]), &heard, 1, "");
    let (added, exchanges, budget) = (
        p.find("also check the tests").unwrap(),
        p.find("TOOL EXCHANGES SO FAR").unwrap(),
        p.find("You may make 1 more").unwrap(),
    );
    assert!(exchanges < added && added < budget, "{p}");
}

/// An engine's line on how to end the answer closes every follow-up, the
/// last one's too, so the follow-up cannot disagree with the prompt.
#[test]
fn the_engines_ending_closes_each_follow_up() {
    let end = "end it as HOW TO REPLY says";
    let p = follow_up("TASK", "", &log(&[("a:b", "r")]), "", 2, end);
    assert!(
        p.ends_with("or give your final answer and end it as HOW TO REPLY says."),
        "{p}"
    );
    let p = follow_up("TASK", "", &log(&[("a:b", "r")]), "", 0, end);
    assert!(
        p.ends_with("with what you have and end it as HOW TO REPLY says."),
        "{p}"
    );
    let p = super::super::lastword::prompt("TASK", "", "log", end);
    assert!(
        p.ends_with("could not check and end it as HOW TO REPLY says."),
        "{p}"
    );
}

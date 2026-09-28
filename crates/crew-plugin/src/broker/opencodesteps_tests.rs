use super::super::normalize::opencode_json;

/// Real `opencode run --format json` captures (opencode 1.18.32, run from
/// the repo root on 2026-09-28). THEME and SYSTOOL narrate before a tool
/// call; THEME's `read` also carries the whole file under a nested `text`.
const THEME: &str = include_str!("testdata/opencode_theme.jsonl");
const SYSTOOL: &str = include_str!("testdata/opencode_systool.jsonl");
const TASKCAP: &str = include_str!("testdata/opencode_taskcap.jsonl");
/// A run that ended on its first tool call: the broker handed opencode a
/// stale `PWD`, so reading the project's own file was refused and the
/// stream is step, refused `read`, finish — no text anywhere.
const REFUSED: &str = include_str!("testdata/opencode_refused.jsonl");

const START: &str = r#"{"type":"step_start","part":{"type":"step-start"}}"#;
const CALLS: &str = r#"{"type":"step_finish","part":{"type":"step-finish","reason":"tool-calls"}}"#;
const STOP: &str = r#"{"type":"step_finish","part":{"type":"step-finish","reason":"stop"}}"#;

fn text(t: &str) -> String {
    format!(r#"{{"type":"text","part":{{"type":"text","text":"{t}"}}}}"#)
}

fn stream(lines: &[&str]) -> String {
    lines.join("\n")
}

#[test]
fn narration_before_a_tool_call_is_not_the_reply() {
    let raw = stream(&[
        START,
        &text("Let me look at the definitions."),
        r#"{"type":"tool_use","part":{"type":"tool","tool":"grep","state":{"output":"a.rs:1"}}}"#,
        CALLS,
        START,
        &text("There are three definitions."),
        STOP,
    ]);
    assert_eq!(opencode_json(&raw), "There are three definitions.");
}

#[test]
fn text_inside_a_tool_state_contributes_nothing() {
    let tool = r#"{"type":"tool_use","part":{"type":"tool","tool":"read","state":{
        "output":"//! Crew's color themes.","text":"FILE BODY",
        "metadata":{"display":{"text":"FILE BODY"}}}}}"#
        .replace('\n', "");
    let raw = stream(&[START, &tool, &text("It exports Theme."), STOP]);
    assert_eq!(opencode_json(&raw), "It exports Theme.");
}

#[test]
fn a_reasoning_part_contributes_nothing() {
    let reasoning = r#"{"type":"reasoning","part":{"type":"reasoning","text":"The user is asking. Wait, let me re-read."}}"#;
    let raw = stream(&[START, reasoning, &text("src/lib.rs"), STOP]);
    assert_eq!(opencode_json(&raw), "src/lib.rs");
}

#[test]
fn a_stream_ending_on_a_tool_call_keeps_its_latest_narration() {
    let raw = stream(&[
        START,
        &text("First I'll grep."),
        CALLS,
        START,
        &text("Let me check one more file."),
        CALLS,
        START,
        r#"{"type":"tool_use","part":{"type":"tool","tool":"read","state":{"output":"x"}}}"#,
    ]);
    assert_eq!(opencode_json(&raw), "Let me check one more file.");
}

#[test]
fn a_step_stream_with_no_text_still_surfaces_its_error() {
    let raw = stream(&[
        START,
        r#"{"type":"tool_use","part":{"type":"tool","tool":"read","state":{"text":"x"}}}"#,
        r#"{"type":"error","error":{"name":"APIError","data":{"message":"rate limited"}}}"#,
    ]);
    assert_eq!(opencode_json(&raw), "[opencode error] rate limited");
}

#[test]
fn real_stream_answers_without_its_narration_or_the_file_it_read() {
    assert!(
        THEME.contains(r#""text":"I'll read that file.""#),
        "fixture lost its narration"
    );
    assert!(
        THEME.contains(r#""text":"//! Crew's color themes."#),
        "fixture lost the file"
    );
    assert_eq!(
        opencode_json(THEME),
        concat!(
            "It exports the `Theme` struct (every UI colour) with its 12 `&'static` ",
            "presets, the `ThemeId`/`RandomMode`/`Selection` theme-selection enums plus ",
            "parsing and global rotation/active-theme state (`set_theme`, `theme`, ",
            "`tick_random`, `apply_selection`, …), WCAG contrast helpers, tag-colour and ",
            "font/favourite utilities, and ~14 public submodules (`ramp`, `ansi`, ",
            "`oklch`, `deco`, `glassborder`, …).",
        )
    );
}

#[test]
fn real_stream_of_the_live_question_answers_with_its_final_step() {
    let reply = opencode_json(SYSTOOL);
    assert!(
        SYSTOOL.contains("I'll explore the codebase"),
        "fixture lost its narration"
    );
    assert!(!reply.contains("I'll explore"), "narration leaked: {reply}");
    assert!(!reply.contains("<task_result>"), "subagent output leaked");
    assert_eq!(
        reply.lines().next(),
        Some(
            "Adding a `sys:` tool means adding a tool to the broker's `sys` server \
             (that's what agent smith exposes). Minimum recipe — 5 code files:"
        )
    );
    assert!(reply.ends_with("Precedent commit: `40e752e1` (added `sys:edit`, 12 files)."));
}

#[test]
fn real_stream_of_tool_steps_with_no_narration_answers_with_the_stop_step() {
    assert_eq!(
        opencode_json(TASKCAP),
        "crates/crew-plugin/src/broker/toolchoice.rs, crates/crew-plugin/src/broker/route.rs, \
         crates/crew-plugin/src/broker/skillgrammar.rs (module-local consts, not a single definition)"
    );
}

#[test]
fn a_real_stream_that_never_answers_says_so_in_one_line() {
    assert_eq!(REFUSED.lines().count(), 3, "fixture lost an event");
    let reply = opencode_json(REFUSED);
    assert!(!reply.contains(r#"{"type""#), "raw stream leaked: {reply}");
    assert_eq!(
        reply,
        "opencode stopped after 1 step without answering (it ran: read; read failed)"
    );
}

#[test]
fn a_silent_stream_counts_its_steps_and_names_each_tool_once() {
    let bash = r#"{"type":"tool_use","part":{"type":"tool","tool":"bash","state":{"status":"completed"}}}"#;
    let read = r#"{"type":"tool_use","part":{"type":"tool","tool":"read","state":{"status":"completed"}}}"#;
    let raw = stream(&[START, bash, CALLS, START, read, CALLS, START, bash]);
    assert_eq!(
        opencode_json(&raw),
        "opencode stopped after 3 steps without answering (it ran: bash, read)"
    );
}

use std::sync::Arc;

use super::*;
use crate::broker::hop::HopKind;
use crate::broker::toolclip::clip_result;
use crate::broker::turnkit::{turn, Script, Turn};
use crew_hive::tools::parse_tool_call;

struct FakeTools(Result<String, String>);

impl ToolRunner for FakeTools {
    fn hint(&self) -> String {
        "TOOLS: fs:read".into()
    }
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        assert_eq!((server, tool), ("fs", "read"));
        assert!(args.contains("path"), "args: {args}");
        self.0.clone()
    }
}

/// One turn of `replies` with `tools` on the default round budget.
fn with(tools: impl ToolRunner + 'static, replies: &[&str]) -> Turn {
    let tools: Arc<dyn ToolRunner> = Arc::new(tools);
    turn(Script::new(replies), |b| b.with_tools(tools), "task")
}

const READ: &str = "@tool fs:read {\"path\": \"x\"}";

#[test]
fn parse_tool_call_reads_the_last_line() {
    let c = parse_tool_call("thinking\n@tool fs:read {\"path\": \"x\"}").unwrap();
    assert_eq!((c.server.as_str(), c.tool.as_str()), ("fs", "read"));
    assert_eq!(c.args, "{\"path\": \"x\"}");
}

#[test]
fn parse_tool_call_tolerates_wrappers_and_case() {
    let c = parse_tool_call("done soon\n**@Tool `fs:read` {}**").unwrap();
    assert_eq!((c.server.as_str(), c.tool.as_str()), ("fs", "read"));
}

#[test]
fn parse_tool_call_rejects_non_directives() {
    assert!(parse_tool_call("just an answer\n@done").is_none());
    assert!(parse_tool_call("@tool malformed-no-colon {}").is_none());
    assert!(parse_tool_call("").is_none());
}

#[test]
fn the_hint_is_empty_only_when_there_is_nothing_to_call() {
    struct NoTools;
    impl ToolRunner for NoTools {
        fn hint(&self) -> String {
            String::new()
        }
        fn call(&self, _s: &str, _t: &str, _a: &str) -> Result<String, String> {
            unreachable!()
        }
    }
    assert_eq!(hint("task", None), "");
    assert_eq!(hint("task", Some(&NoTools)), "");
    assert!(hint("task", Some(&FakeTools(Ok("x".into())))).contains("fs:read"));
}

#[test]
fn hint_for_lists_each_tool_once() {
    assert_eq!(hint_for(&[]), "");
    let h = hint_for(&[crate::mcp::McpTool {
        server: "fs".into(),
        name: "read".into(),
        description: "Read a file".into(),
        input_schema: serde_json::json!({"type": "object"}),
    }]);
    assert!(h.contains("- fs:read \u{2014} Read a file"), "got: {h}");
    assert!(h.contains("@tool"), "directive syntax is explained");
}

/// Descriptions are now stored WHOLE (native tool-use wants the detail), so
/// the one-line-per-tool list has to do the shortening itself or a server with
/// a paragraph of documentation breaks the list it is listed in.
#[test]
fn a_multi_line_description_stays_one_line_in_the_hint() {
    let h = hint_for(&[crate::mcp::McpTool {
        server: "fs".into(),
        name: "read".into(),
        description: "Read a file\n\nSupports offsets, globs, and\nmuch more besides.".into(),
        input_schema: serde_json::json!({"type": "object"}),
    }]);
    let listed: Vec<&str> = h.lines().filter(|l| l.starts_with("- ")).collect();
    assert_eq!(listed.len(), 1, "{h}");
    assert!(!h.contains("much more besides"), "{h}");
}

#[test]
fn a_tool_result_goes_back_and_the_turn_answers() {
    let t = with(
        FakeTools(Ok("FILE CONTENTS".into())),
        &[&format!("let me look\n{READ}"), "used the file\n@done"],
    );
    assert_eq!(t.answer(), Some("used the file"));
    assert_eq!(t.stats.exchanges, 2);
    let cards = t.cards();
    assert!(
        cards.iter().any(|c| c.contains("[tool] fs:read")),
        "{cards:?}"
    );
    assert!(
        cards.iter().any(|c| c.contains("FILE CONTENTS")),
        "{cards:?}"
    );
    assert!(
        t.prompts[1].contains("RESULT:\nFILE CONTENTS"),
        "{}",
        t.prompts[1]
    );
}

#[test]
fn a_tool_error_is_shown_to_the_agent_and_the_turn_goes_on() {
    let t = with(
        FakeTools(Err("no such file".into())),
        &[&format!("trying\n{READ}"), "cannot read it\n@done"],
    );
    assert_eq!(t.answer(), Some("cannot read it"));
    let cards = t.cards();
    assert!(
        cards.iter().any(|c| c.contains("ERROR: no such file")),
        "{cards:?}"
    );
}

/// An agent that asks every time: its rounds, the batch past them, and one
/// last call with no tools for the answer, which says the budget ran out.
#[test]
fn the_round_cap_ends_on_the_last_word_and_says_so() {
    let ran = Arc::new(std::sync::Mutex::new(0u32));
    struct Counting(Arc<std::sync::Mutex<u32>>);
    impl ToolRunner for Counting {
        fn hint(&self) -> String {
            "TOOLS: fs:read".into()
        }
        fn call(&self, _: &str, _: &str, _: &str) -> Result<String, String> {
            *self.0.lock().unwrap() += 1;
            Ok("more".into())
        }
    }
    let t = with(Counting(Arc::clone(&ran)), &[&format!("again\n{READ}")]);
    assert_eq!(
        *ran.lock().unwrap(),
        MAX_TOOL_ROUNDS + 1,
        "the batch past the budget ran"
    );
    assert_eq!(
        t.stats.exchanges,
        MAX_TOOL_ROUNDS + 2,
        "the dials, then the last word"
    );
    let answer = t.answer().unwrap();
    assert!(!answer.contains("@tool"), "{answer}");
    assert!(answer.starts_with("again"), "{answer}");
    assert!(answer.contains("tool budget spent"), "{answer}");
    let last = t.prompts.last().unwrap();
    assert!(last.contains("Your tool budget is spent"), "{last}");
    assert!(
        !last.contains("TOOLS: fs:read"),
        "no tools on the last word: {last}"
    );
}

/// An agent that finishes inside its budget is told nothing about budgets —
/// the note is for the turn that actually hit the wall.
#[test]
fn finishing_within_the_budget_says_nothing_about_it() {
    let t = with(
        FakeTools(Ok("contents".into())),
        &[&format!("let me look\n{READ}"), "all done\n@done"],
    );
    assert_eq!(t.answer(), Some("all done"));
    assert!(t.of(HopKind::Terminated).is_empty(), "{:?}", t.hops);
}

/// The agent is told what it has left, and told plainly when the next call
/// would be its last. A budget it cannot see is one it plans straight past.
#[test]
fn the_follow_up_prompt_states_the_remaining_budget() {
    let t = with(
        FakeTools(Ok("contents".into())),
        &[&format!("again\n{READ}")],
    );
    let follow_ups = &t.prompts[1..=MAX_TOOL_ROUNDS as usize];
    assert!(
        follow_ups[0].contains("more round(s) of tool calls"),
        "the first follow-up must state the remaining budget: {}",
        follow_ups[0]
    );
    let last = follow_ups.last().unwrap();
    assert!(last.contains("LAST round of tool calls"), "{last}");
}

/// With no tool surface a hop is one dial, and a `@tool` line is just text.
#[test]
fn without_tools_a_hop_is_one_dial() {
    let t = turn(Script::new(&["answer\n@tool fs:read {}"]), |b| b, "task");
    assert_eq!(t.prompts.len(), 1);
    assert_eq!(t.stats.exchanges, 1);
    assert!(t.answer().unwrap().starts_with("answer\n@tool fs:read {}"));
}

/// A runner that routes `sys:` to the real built-in tools (the session
/// bridge's local arm), proving the engine loop executes real commands.
struct SysOnly;

impl ToolRunner for SysOnly {
    fn hint(&self) -> String {
        hint_for(&crate::broker::systools::tools())
    }
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        assert_eq!(server, "sys");
        crate::broker::systools::call(tool, args)
    }
}

#[test]
fn relay_runs_a_real_sys_command_and_logs_hops() {
    let t = with(
        SysOnly,
        &[
            "@tool sys:run {\"cmd\":\"echo tool-e2e\"}",
            "the command printed tool-e2e",
        ],
    );
    assert_eq!(t.answer(), Some("the command printed tool-e2e"));
    // The call and its result go out on the tool lane, as Claude Code's do;
    // the pane's tool line reads the command out of the arguments.
    let cards = t.cards();
    assert!(
        cards[0].starts_with("[tool] sys:run ") && cards[0].contains("echo tool-e2e"),
        "{cards:?}"
    );
    assert!(cards[1].contains("tool-e2e"), "result logged: {cards:?}");
}

#[test]
fn pointer_framed_skill_lets_the_agent_read_the_playbook() {
    // A >8 KB skill on disk: the frame must carry a pointer, and the loop
    // must resolve a sys:read_file for that pointer's path.
    let p = std::env::temp_dir().join(format!("crew-skillframe-e2e-{}.md", std::process::id()));
    let body = format!(
        "Intro.\n## Only Section\n{}",
        "needle-content\n".repeat(700)
    );
    std::fs::write(&p, &body).unwrap();
    let mut skill = crate::broker::skills::parse(&body, "big-skill", "user");
    skill.path = p.clone();
    let frame = crate::broker::skillframe::framed(&skill, "use the playbook", true);
    assert!(frame.contains("Full playbook:"), "got: {frame}");
    assert!(frame.contains(&p.display().to_string()), "got: {frame}");

    let first = format!(
        "checking\n@tool sys:read_file {{\"path\": \"{}\"}}",
        p.display()
    );
    let tools: Arc<dyn ToolRunner> = Arc::new(SysOnly);
    let t = turn(
        Script::new(&[&first, "read it, proceeding"]),
        |b| b.with_tools(tools),
        &frame,
    );
    assert_eq!(t.answer(), Some("read it, proceeding"));
    assert!(
        t.cards().iter().any(|c| c.contains("needle-content")),
        "the tool result carries the playbook text"
    );
    let _ = std::fs::remove_file(&p);
}

/// A big `sys:read_file`-shaped result is cut from the middle: its lines
/// stay lines, and the notice on its last line, which the agent needs
/// verbatim to read on, survives.
#[test]
fn a_large_result_keeps_its_newlines_and_its_final_line() {
    let mut body = String::new();
    for i in 0..2000 {
        body.push_str(&format!("line {i}: some padding text here\n"));
    }
    let notice = "\u{2026} (lines 1\u{2013}143 of 912, bytes 0\u{2013}5,480 of 40,112 \u{2014} \
                  continue with {\"offset\": 5480})";
    body.push_str(notice);
    assert!(body.len() > 6000, "fixture must exceed the clip budget");
    let t = with(
        FakeTools(Ok(body)),
        &[&format!("reading\n{READ}"), "got it\n@done"],
    );
    let follow = &t.prompts[1];
    let result = follow.split("RESULT:\n").nth(1).unwrap();
    assert!(result.contains("line 0: "), "the head: {result}");
    assert!(
        result.contains(notice),
        "the final line, verbatim: {result}"
    );
    assert!(result.chars().count() < 6_500, "{}", result.chars().count());
}

#[test]
fn clip_result_keeps_the_final_line_when_it_alone_fits_the_budget() {
    // Final line is UNDER max but within the marker's width of it (the "bad
    // band"): the old `reserved >= max` check fired here and hard-capped,
    // dropping the verbatim final line the continuation protocol needs.
    let max = 6000usize;
    let head = "line one\nline two\n".repeat(50);
    let last_line = "y".repeat(max - 10);
    let text = format!("{head}{last_line}");
    assert!(text.chars().count() > max, "fixture must exceed the budget");

    let clipped = clip_result(&text, max);
    assert!(
        clipped.ends_with(&last_line),
        "final line must survive verbatim when it alone fits the budget, got tail: {}",
        &clipped[clipped.len().saturating_sub(50)..]
    );
}

#[test]
fn a_single_line_result_is_held_to_the_budget() {
    // Minified/base64-shaped output: no newline at all, so the cut falls in
    // the line, and the result is still held near the 6,000 budget.
    let t = with(
        FakeTools(Ok("x".repeat(7000))),
        &[&format!("reading\n{READ}"), "got it\n@done"],
    );
    let follow = &t.prompts[1];
    let longest = follow.split(|c| c != 'x').map(str::len).max().unwrap();
    assert!(longest <= 6_000, "{longest}");
}

/// A result says whether it worked and carries what the tool said: the
/// pane's tool line marks it ✓ or ✗ and opens the text on a click.
#[test]
fn a_relay_tool_result_is_marked_the_way_the_swarm_marks_it() {
    let t = with(
        FakeTools(Ok("FILE CONTENTS".into())),
        &["reading\n@tool fs:read {\"path\":\"x\"}", "done\n@done"],
    );
    let cards = t.cards();
    let result = cards
        .iter()
        .find(|c| c.starts_with("[tool] fs:read \u{2713}"));
    let mut lines = result.expect("a passed result").lines();
    lines.next();
    assert_eq!(lines.next(), Some("FILE CONTENTS"));
}

#[test]
fn a_relay_tool_failure_carries_the_failure_mark() {
    let t = with(
        FakeTools(Err("connection refused".into())),
        &["trying\n@tool fs:read {\"path\":\"x\"}", "gave up\n@done"],
    );
    let cards = t.cards();
    let result = cards
        .iter()
        .find(|c| c.starts_with("[tool] fs:read \u{2717}"));
    assert!(result
        .expect("a failed result")
        .contains("connection refused"));
}

/// A follow-up that fails ends the turn on what the agent had said, with a
/// line saying why — never on its raw `@tool` line, which the caller would
/// otherwise show as the answer (inventory 2026-10-16, item 7).
#[test]
fn a_failed_follow_up_never_leaves_the_raw_call_as_the_answer() {
    for (then, why) in [
        (Err("connection reset".to_string()), "connection reset"),
        (Ok("  ".to_string()), "an empty reply"),
    ] {
        let first = Ok(format!("Let me look at x.\n{READ}"));
        let tools: Arc<dyn ToolRunner> = Arc::new(FakeTools(Ok("FILE CONTENTS".into())));
        let t = turn(
            Script::of(vec![first, then]),
            |b| b.with_tools(tools),
            "task",
        );
        let answer = t.answer().unwrap();
        assert!(
            !answer.contains("@tool"),
            "raw call as the answer: {answer:?}"
        );
        assert!(answer.starts_with("Let me look at x."), "{answer:?}");
        assert!(answer.contains(why), "says why it stopped: {answer:?}");
        assert_eq!(
            t.of(HopKind::Error).len(),
            1,
            "and the error card: {:?}",
            t.hops
        );
    }
}

/// A finished answer that still holds a call says the call never ran
/// (baseline probe 2026-10-16: a fenced `@tool sys:git` under `@done`).
#[test]
fn an_answer_still_holding_a_call_says_it_never_ran() {
    let a = "@done\n\nWe will look at history:\n```json\n@tool sys:git {\"cmd\":\"log\"}\n```\nThen fix it.";
    let got = crate::broker::toolround::never_ran(a.to_string());
    assert!(got.starts_with(a), "the answer as written: {got}");
    assert!(got.ends_with("not run \u{2014} the answer asks for `sys:git`, but the turn ended before any call in it ran"), "{got}");
    assert_eq!(crate::broker::toolround::never_ran("Done.".into()), "Done.");
    // Through the seam every final answer passes (`engine::done`).
    let done = crate::broker::toolround::finished(None, a.to_string());
    assert!(done.contains("not run \u{2014}"), "{done}");
}

/// Crew's own tools are listed whole on the text path: clipped at 100,
/// sys:edit's line lost `edits`, `all` and `line`, and 3 of 3 first edits in
/// one probe were refused for want of the `line` it offers. Another
/// server's stays clipped.
#[test]
fn crews_own_tools_are_listed_whole() {
    let h = hint_for(&crate::broker::systools::tools());
    let edit = h.lines().find(|l| l.starts_with("- sys:edit ")).unwrap();
    for word in ["\"edits\"", "\"all\": true", "\"line\": N"] {
        assert!(edit.contains(word), "{word} in {edit}");
    }
    let long = "x".repeat(300);
    let other = hint_for(&[crate::mcp::McpTool {
        server: "fs".into(),
        name: "read".into(),
        description: long,
        input_schema: serde_json::json!({"type": "object"}),
    }]);
    let line = other.lines().find(|l| l.starts_with("- fs:read")).unwrap();
    assert!(line.chars().count() < 130, "{line}");
}

/// Three reads on a reply's last lines run as one round: one follow-up
/// carries all three results, and one round of the budget is spent.
#[test]
fn a_replys_batch_is_one_round() {
    let reads = "three files\n@tool fs:read {\"path\": \"a\"}\n@tool fs:read {\"path\": \"b\"}\n\
                 @tool fs:read {\"path\": \"c\"}";
    let t = with(
        FakeTools(Ok("FILE".into())),
        &[reads, "read all three\n@done"],
    );
    assert_eq!(t.prompts.len(), 2, "one follow-up for the three");
    let follow = &t.prompts[1];
    assert_eq!(follow.matches("RESULT:\nFILE").count(), 3, "{follow}");
    assert!(
        follow.contains(&format!(
            "You may make {} more round(s)",
            MAX_TOOL_ROUNDS - 1
        )),
        "{follow}"
    );
    assert_eq!(t.answer(), Some("read all three"));
}

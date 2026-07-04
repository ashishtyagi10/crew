use std::sync::Mutex;
use std::time::Duration;

use super::*;
use crate::Registry;

/// An agent whose replies are scripted; repeats the last one when exhausted.
struct Scripted(Mutex<Vec<String>>);

impl Scripted {
    fn new(replies: &[&str]) -> Self {
        let mut v: Vec<String> = replies.iter().rev().map(|s| s.to_string()).collect();
        v.shrink_to_fit();
        Self(Mutex::new(v))
    }
}

impl Adapter for Scripted {
    fn name(&self) -> &str {
        "planner"
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, _body: &str, _t: Duration) -> Result<String, String> {
        let mut v = self.0.lock().unwrap();
        Ok(match v.len() {
            0 => "@done".into(),
            1 => v[0].clone(),
            _ => v.pop().unwrap(),
        })
    }
}

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

fn broker_with(runner: FakeTools) -> Broker {
    Broker::new(Registry::new(vec![]), 6, Duration::from_secs(5))
        .with_tools(std::sync::Arc::new(runner))
}

fn env() -> Envelope {
    Envelope::new("user", "planner", "t1", "task")
}

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
fn augment_appends_the_hint_only_when_tools_exist() {
    struct NoTools;
    impl ToolRunner for NoTools {
        fn hint(&self) -> String {
            String::new()
        }
        fn call(&self, _s: &str, _t: &str, _a: &str) -> Result<String, String> {
            unreachable!()
        }
    }
    assert_eq!(augment("task", None), "task");
    assert_eq!(augment("task", Some(&NoTools)), "task");
    let with = augment("task", Some(&FakeTools(Ok("x".into()))));
    assert!(with.starts_with("task\n\n") && with.contains("fs:read"));
}

#[test]
fn hint_for_lists_each_tool_once() {
    assert_eq!(hint_for(&[]), "");
    let h = hint_for(&[crate::mcp::McpTool {
        server: "fs".into(),
        name: "read".into(),
        description: "Read a file".into(),
    }]);
    assert!(h.contains("- fs:read \u{2014} Read a file"), "got: {h}");
    assert!(h.contains("@tool"), "directive syntax is explained");
}

#[test]
fn run_tools_feeds_the_result_back_and_returns_the_final_reply() {
    let b = broker_with(FakeTools(Ok("FILE CONTENTS".into())));
    let agent = Scripted::new(&["used the file\n@done"]);
    let mut stats = RunStats::default();
    let mut hops = Vec::new();
    let reply = b.run_tools(
        &agent,
        "base prompt",
        "let me look\n@tool fs:read {\"path\": \"x\"}".into(),
        &mut stats,
        &env(),
        &mut |h| hops.push(h),
    );
    assert_eq!(reply, "used the file\n@done");
    assert_eq!(stats.exchanges, 1);
    assert!(hops.iter().any(|h| h.text.contains("[tool] fs:read")));
    assert!(hops.iter().any(|h| h.text.contains("FILE CONTENTS")));
}

#[test]
fn run_tools_shows_errors_to_the_agent_and_continues() {
    let b = broker_with(FakeTools(Err("no such file".into())));
    let agent = Scripted::new(&["cannot read it\n@done"]);
    let mut stats = RunStats::default();
    let mut hops = Vec::new();
    let reply = b.run_tools(
        &agent,
        "base",
        "trying\n@tool fs:read {\"path\": \"x\"}".into(),
        &mut stats,
        &env(),
        &mut |h| hops.push(h),
    );
    assert_eq!(reply, "cannot read it\n@done");
    assert!(hops.iter().any(|h| h.text.contains("ERROR: no such file")));
}

#[test]
fn run_tools_stops_at_the_round_cap() {
    let b = broker_with(FakeTools(Ok("more".into())));
    // The agent asks for a tool every single time.
    let agent = Scripted::new(&["again\n@tool fs:read {\"path\": \"x\"}"]);
    let mut stats = RunStats::default();
    let reply = b.run_tools(
        &agent,
        "base",
        "again\n@tool fs:read {\"path\": \"x\"}".into(),
        &mut stats,
        &env(),
        &mut |_| {},
    );
    assert_eq!(stats.exchanges, MAX_TOOL_ROUNDS);
    assert!(reply.contains("@tool"), "cap leaves the last reply as-is");
}

#[test]
fn run_tools_without_a_runner_is_a_no_op() {
    let b = Broker::new(Registry::new(vec![]), 6, Duration::from_secs(5));
    let agent = Scripted::new(&[]);
    let mut stats = RunStats::default();
    let reply = b.run_tools(
        &agent,
        "base",
        "answer\n@tool fs:read {}".into(),
        &mut stats,
        &env(),
        &mut |_| {},
    );
    assert_eq!(reply, "answer\n@tool fs:read {}");
    assert_eq!(stats.exchanges, 0);
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
    let broker = Broker::new(Registry::new(vec![]), 6, Duration::from_secs(5))
        .with_tools(std::sync::Arc::new(SysOnly));
    let agent = Scripted::new(&[
        "@tool sys:run {\"cmd\":\"echo tool-e2e\"}",
        "the command printed tool-e2e",
    ]);
    let mut hops = Vec::new();
    let mut stats = RunStats::default();
    let reply = broker.run_tools(
        &agent,
        "task",
        "@tool sys:run {\"cmd\":\"echo tool-e2e\"}".into(),
        &mut stats,
        &env(),
        &mut |h| hops.push(h),
    );
    assert_eq!(reply, "the command printed tool-e2e");
    assert!(
        hops.iter().any(|h| h.text.contains("[tool] sys:run")),
        "call hop logged"
    );
    assert!(
        hops.iter().any(|h| h.text.contains("tool-e2e")),
        "result hop logged"
    );
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

    let broker = Broker::new(Registry::new(vec![]), 6, Duration::from_secs(5))
        .with_tools(std::sync::Arc::new(SysOnly));
    let agent = Scripted::new(&["read it, proceeding"]);
    let mut hops = Vec::new();
    let mut stats = RunStats::default();
    let reply = broker.run_tools(
        &agent,
        &frame,
        format!(
            "checking\n@tool sys:read_file {{\"path\": \"{}\"}}",
            p.display()
        ),
        &mut stats,
        &env(),
        &mut |h| hops.push(h),
    );
    assert_eq!(reply, "read it, proceeding");
    assert!(
        hops.iter().any(|h| h.text.contains("needle-content")),
        "tool result hop carries the playbook text"
    );
    let _ = std::fs::remove_file(&p);
}

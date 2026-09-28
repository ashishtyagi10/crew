//! The tools section reaches the agent however long the task is.
//!
//! Measured on a live DashScope run (2026-09-27): a nine-kilobyte skill
//! roster headed the task, the relay clipped the task at `TASK_CAP`, and the
//! `@tool` section — appended at the END of the task — went with the clip.
//! Asked what a crate in the repo does, the agent had no way to read the repo
//! and described a web framework that does not exist.
use super::*;
use crate::broker::toolcall::ToolRunner;
use crate::broker::Adapter;
use std::sync::{Arc, Mutex};

struct Recorder(Arc<Mutex<Vec<String>>>);

impl Adapter for Recorder {
    fn name(&self) -> &str {
        "claude"
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: std::time::Duration) -> Result<String, String> {
        self.0.lock().unwrap().push(body.to_string());
        Ok("done reading\n@done".into())
    }
}

struct ReadTools;

impl ToolRunner for ReadTools {
    fn hint(&self) -> String {
        "TOOLS (optional): @tool sys:read_file {\"path\": …}".into()
    }
    fn call(&self, _s: &str, _t: &str, _a: &str) -> Result<String, String> {
        unreachable!("the scripted agent calls no tool")
    }
}

fn first_prompt(body: &str) -> String {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let agent: Box<dyn Adapter> = Box::new(Recorder(Arc::clone(&seen)));
    let b = Broker::new(
        Registry::new(vec![agent]),
        2,
        std::time::Duration::from_secs(1),
    )
    .with_tools(Arc::new(ReadTools));
    b.run(
        "user",
        "claude",
        body,
        "t1",
        &crate::broker::tick::noop_tick_emit(),
        &mut |_| {},
    );
    let first = seen.lock().unwrap().first().cloned();
    first.expect("the agent was dialed")
}

#[test]
fn a_task_longer_than_the_cap_still_carries_its_tools() {
    let long = format!(
        "{}\n\nwhat does crew-render do?",
        "roster line. ".repeat(900)
    );
    assert!(long.len() > 4000 * 2, "the body is well past TASK_CAP");
    let p = first_prompt(&long);
    assert!(
        p.contains("@tool sys:read_file"),
        "tools clipped away:\n{}",
        &p[..600]
    );
}

#[test]
fn a_short_task_carries_them_too() {
    let p = first_prompt("what does crew-render do?");
    assert!(p.contains("@tool sys:read_file"), "{p}");
}

/// …and says to use them before answering about the project: offered as
/// merely optional, the model answered from its priors with the tools listed.
#[test]
fn the_tools_section_says_look_before_answering() {
    let t = crate::mcp::McpTool {
        server: "sys".into(),
        name: "read_file".into(),
        description: "read a file".into(),
        input_schema: serde_json::json!({"type": "object"}),
    };
    let h = crate::broker::toolcall::hint_for(&[t]);
    assert!(h.contains("LOOK FIRST"), "{h}");
    assert!(h.contains("Never describe code you have not read"), "{h}");
}

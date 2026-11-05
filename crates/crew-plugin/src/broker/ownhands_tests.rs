//! An agent with hands of its own is not offered crew's copies of them.
use super::{for_own_hands, OwnHands};
use crate::broker::toolcall::{hint_for, ToolRunner};
use crate::broker::{Adapter, Broker, Registry};
use crate::mcp::McpTool;
use std::sync::{Arc, Mutex};

fn tool(server: &str, name: &str) -> McpTool {
    McpTool {
        server: server.into(),
        name: name.into(),
        description: format!("{name} does its thing"),
        input_schema: serde_json::json!({"type": "object"}),
    }
}

/// The section a session with crew's file tools, `sys:todo` and one MCP
/// server hands every hop.
fn full_hint() -> String {
    let names = ["read_file", "write_file", "edit", "run", "grep", "glob"];
    let mut tools: Vec<McpTool> = names.iter().map(|n| tool("sys", n)).collect();
    tools.push(tool("sys", "todo"));
    tools.push(tool("github", "create_issue"));
    hint_for(&tools)
}

#[test]
fn the_twins_and_look_first_go_and_the_rest_stay() {
    let h = for_own_hands(&full_hint());
    for gone in [
        "sys:read_file",
        "sys:write_file",
        "sys:edit ",
        "sys:run ",
        "LOOK FIRST",
    ] {
        assert!(!h.contains(gone), "{gone} still offered:\n{h}");
    }
    assert!(h.starts_with("TOOLS:"), "{h}");
    assert!(h.contains("- sys:todo"), "{h}");
    assert!(h.contains("- github:create_issue"), "{h}");
}

#[test]
fn nothing_but_twins_is_no_section_at_all() {
    let only = hint_for(&[tool("sys", "read_file"), tool("sys", "run")]);
    assert_eq!(for_own_hands(&only), "");
}

/// An agent that writes a file through crew on its first reply, then ends.
struct Recorder {
    seen: Arc<Mutex<Vec<String>>>,
    own_hands: bool,
}

impl Adapter for Recorder {
    fn name(&self) -> &str {
        "claude"
    }
    fn probe(&self) -> bool {
        true
    }
    fn reads_files(&self) -> bool {
        self.own_hands
    }
    fn call(&self, body: &str, _t: std::time::Duration) -> Result<String, String> {
        let mut seen = self.seen.lock().unwrap();
        seen.push(body.to_string());
        Ok(match seen.len() {
            1 => "writing\n@tool sys:write_file {\"path\": \"n.txt\", \"content\": \"hi\"}".into(),
            _ => "done\n@done".into(),
        })
    }
}

/// The session's tools: what reaches it is what crew ran.
struct Session(Arc<Mutex<Vec<String>>>);

impl ToolRunner for Session {
    fn hint(&self) -> String {
        full_hint()
    }
    fn call(&self, s: &str, t: &str, _a: &str) -> Result<String, String> {
        self.0.lock().unwrap().push(format!("{s}:{t}"));
        Ok(match t {
            "find_tools" => {
                "- sys:write_file \u{2014} write\n- github:create_issue \u{2014} file".into()
            }
            _ => "wrote 2 bytes".into(),
        })
    }
}

/// The prompts the agent was sent, and the calls crew ran for it.
fn turn(own_hands: bool) -> (Vec<String>, Vec<String>) {
    let (seen, ran) = (Arc::default(), Arc::default());
    let agent = Recorder {
        seen: Arc::clone(&seen),
        own_hands,
    };
    let b = Broker::new(
        Registry::new(vec![Box::new(agent) as Box<dyn Adapter>]),
        2,
        std::time::Duration::from_secs(5),
    )
    .with_tools(Arc::new(Session(Arc::clone(&ran))));
    let tick = crate::broker::tick::noop_tick_emit();
    b.run("user", "claude", "create n.txt", "t1", &tick, &mut |_| {});
    let prompts = seen.lock().unwrap().clone();
    let calls = ran.lock().unwrap().clone();
    (prompts, calls)
}

/// Loop 8's live run: Claude Code, denied its own Write, answered with
/// `@tool sys:write_file` — because its prompt had offered it.
#[test]
fn a_hop_to_an_agent_with_its_own_hands_never_offers_crews() {
    let (prompts, _) = turn(true);
    let p = &prompts[0];
    assert!(!p.contains("sys:write_file"), "{p}");
    assert!(!p.contains("LOOK FIRST"), "{p}");
    assert!(p.contains("- sys:todo"), "{p}");
    assert!(p.contains("- github:create_issue"), "{p}");
}

/// Live, with the list trimmed, Claude Code found `sys:write_file` through
/// `sys:find_tools` and wrote through it: the call itself must not run.
#[test]
fn a_twin_called_anyway_is_refused_with_the_reason() {
    let (prompts, calls) = turn(true);
    assert!(calls.is_empty(), "crew ran {calls:?}");
    let after = prompts.get(1).expect("the refusal went back to the agent");
    assert!(after.contains("tell the user what was refused"), "{after}");
}

#[test]
fn a_model_without_hands_keeps_them() {
    let (prompts, calls) = turn(false);
    assert!(prompts[0].contains("- sys:write_file"), "{}", prompts[0]);
    assert!(prompts[0].contains("LOOK FIRST"), "{}", prompts[0]);
    assert_eq!(calls, ["sys:write_file"]);
}

#[test]
fn find_tools_never_finds_a_twin() {
    let own = OwnHands(Arc::new(Session(Arc::default())));
    let found = own.call("sys", "find_tools", "{\"q\": \"write\"}").unwrap();
    assert_eq!(found, "- github:create_issue \u{2014} file");
}

use std::sync::{Arc, Mutex};

use crew_hive::tools::ToolCall;
use crew_hive::HiveEvent;

use super::{carded, send};
use crate::broker::adapter::HopStream;
use crate::broker::toolbatch::Ran;

fn call(server: &str, tool: &str, args: &str) -> ToolCall {
    ToolCall {
        server: server.into(),
        tool: tool.into(),
        args: args.into(),
    }
}

fn ran(ok: bool) -> Ran {
    Ran {
        ok,
        text: String::new(),
        ms: 1,
    }
}

/// A stream whose tool lane keeps what it was handed.
fn captured() -> (Arc<Mutex<Vec<HiveEvent>>>, HopStream) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let keep = log.clone();
    let mut s = HopStream::noop();
    s.on_tool = Arc::new(move |e| keep.lock().unwrap().push(e));
    (log, s)
}

#[test]
fn only_a_checklist_write_loses_its_tool_cards() {
    assert!(!carded(&call("sys", "todo", "{}")));
    assert!(carded(&call("sys", "read_file", "{}")));
    assert!(
        carded(&call("fs", "todo", "{}")),
        "another server's `todo` is a tool"
    );
}

#[test]
fn an_accepted_write_goes_out_as_the_list_the_agent_sent() {
    let (log, s) = captured();
    let args = r#"{"items":[{"text":"fix it","status":"in_progress"}]}"#;
    send(&s, "engineer", &call("sys", "todo", args), &ran(true));
    let evs = log.lock().unwrap();
    assert!(
        matches!(&evs[..], [HiveEvent::ToolCall { label, args: a, .. }] if label == "sys:todo" && a == args),
        "{evs:?}"
    );
}

#[test]
fn a_refused_write_and_any_other_tool_send_nothing() {
    let (log, s) = captured();
    send(&s, "engineer", &call("sys", "todo", "{bad"), &ran(false));
    send(&s, "engineer", &call("sys", "run", "{}"), &ran(true));
    assert!(log.lock().unwrap().is_empty());
}

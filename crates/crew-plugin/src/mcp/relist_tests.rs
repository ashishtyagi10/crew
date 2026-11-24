//! `notifications/tools/list_changed` through a fake server: the next listing
//! is the server's new one, the LOG says what came and went, and a server
//! that says nothing is listed once.
#![cfg(unix)]
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use super::changed_note;
use crate::mcp::{McpHost, McpTool, ServerConfig};

/// A fake server that answers as a real one does: each request with an id
/// gets the next of `groups` (and past the last, the server exits), and a group may follow its reply with a
/// notification (messages in a group are tab-joined; JSON has no raw tabs).
fn server(tag: &str, groups: &[&[&str]]) -> McpHost {
    let path =
        std::env::temp_dir().join(format!("crew-mcp-relist-{tag}-{}.txt", std::process::id()));
    let lines: Vec<String> = groups.iter().map(|g| g.join("\t")).collect();
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    let script = format!(
        "exec 3<'{}'; while IFS= read -r req; do case \"$req\" in *'\"id\"'*) \
         {{ IFS= read -r g <&3 || exit 0; }} && printf '%s\\n' \"$g\" | tr '\\t' '\\n';; esac; done",
        path.display()
    );
    let cfg = ServerConfig {
        command: "sh".into(),
        args: vec!["-c".into(), script],
        env: BTreeMap::new(),
    };
    McpHost::new(BTreeMap::from([("fake".to_string(), cfg)]))
}

/// Every note the host sends to the LOG.
fn notes(host: &mut McpHost) -> Arc<Mutex<Vec<String>>> {
    let log = Arc::new(Mutex::new(Vec::new()));
    let l = Arc::clone(&log);
    host.set_sink(Arc::new(move |_, m: &str| {
        l.lock().unwrap().push(m.to_string())
    }));
    log
}

fn names(host: &mut McpHost) -> Vec<String> {
    host.tools().into_iter().map(|t| t.name).collect()
}

const PROBE: &str =
    r#"{"jsonrpc":"2.0","id":0,"error":{"code":-32601,"message":"no such method"}}"#;
const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{"listChanged":true}},"serverInfo":{"name":"fake"}}}"#;
const ECHO: &str = r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo"}]}}"#;
const CHANGED: &str = r#"{"jsonrpc":"2.0","method":"notifications/tools/list_changed"}"#;

/// The reader thread lands what the server sent between requests a moment
/// after the reply before it.
fn settle() {
    std::thread::sleep(std::time::Duration::from_millis(200));
}

#[test]
fn a_change_announced_between_tasks_is_listed_at_the_next() {
    let both = r#"{"jsonrpc":"2.0","id":3,"result":{"tools":[{"name":"echo"},{"name":"grep"}]}}"#;
    let mut host = server("between", &[&[PROBE], &[INIT], &[ECHO, CHANGED], &[both]]);
    let log = notes(&mut host);
    assert_eq!(names(&mut host), ["echo"]);
    settle();
    assert_eq!(names(&mut host), ["echo", "grep"]);
    let said = log.lock().unwrap().join("\n");
    assert!(
        said.contains("mcp fake tools changed \u{b7} +grep \u{b7} 2 tools"),
        "{said}"
    );
    assert_eq!(
        said.matches("connected").count(),
        1,
        "a relist is not a connect: {said}"
    );
}

/// Heard while waiting for a call's reply — the message the wait used to skip.
#[test]
fn a_change_announced_during_a_call_is_heard() {
    let call = r#"{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"hi"}]}}"#;
    let grep = r#"{"jsonrpc":"2.0","id":4,"result":{"tools":[{"name":"grep"}]}}"#;
    let mut host = server(
        "during",
        &[&[PROBE], &[INIT], &[ECHO], &[CHANGED, call], &[grep]],
    );
    let log = notes(&mut host);
    assert_eq!(names(&mut host), ["echo"]);
    assert_eq!(host.call("fake", "echo", "").unwrap(), "hi");
    let mid = log.lock().unwrap().join("\n");
    assert!(
        !mid.contains("tools changed"),
        "never relisted inside a call: {mid}"
    );
    assert_eq!(
        names(&mut host),
        ["grep"],
        "the removed tool is no longer offered"
    );
    let said = log.lock().unwrap().join("\n");
    assert!(said.contains("+grep \u{2212}echo"), "{said}");
}

/// A server that announces nothing is listed once a session, as before.
#[test]
fn without_a_notice_the_list_is_served_from_the_cache() {
    let other = r#"{"jsonrpc":"2.0","id":3,"result":{"tools":[{"name":"other"}]}}"#;
    let mut host = server("quiet", &[&[PROBE], &[INIT], &[ECHO], &[other]]);
    assert_eq!(names(&mut host), ["echo"]);
    settle();
    assert_eq!(names(&mut host), ["echo"]);
}

#[test]
fn the_note_names_what_came_and_went() {
    let t = |n: &str| McpTool {
        server: "gh".into(),
        name: n.into(),
        description: String::new(),
        input_schema: serde_json::json!({"type": "object"}),
    };
    assert_eq!(
        changed_note("gh", &[t("a"), t("b")], &[t("b"), t("c")]),
        "mcp gh tools changed \u{b7} +c \u{2212}a \u{b7} 2 tools"
    );
    assert_eq!(
        changed_note("gh", &[t("a")], &[t("a")]),
        "mcp gh tools changed \u{b7} 1 tool"
    );
}

/// The server said its tools changed, then died: the listing fails at once
/// ("closed its pipe"). The old list stays — every tool hidden and every
/// call failing until `/reload` was the alternative — and the next read
/// reconnects and lists it.
#[test]
fn a_failed_relist_keeps_the_list_and_retries_on_a_fresh_connection() {
    let mut host = server("fails", &[&[PROBE], &[INIT], &[ECHO, CHANGED]]);
    let log = notes(&mut host);
    assert_eq!(names(&mut host), ["echo"]);
    settle();
    let t = std::time::Instant::now();
    assert_eq!(
        names(&mut host),
        ["echo"],
        "kept through the failed listing"
    );
    assert!(
        t.elapsed().as_secs() < 5,
        "one quick failure, not a timeout"
    );
    assert!(!log.lock().unwrap().join("\n").contains("tools changed"));
    // A fresh process (the fake starts over): listed again, the change said.
    assert_eq!(names(&mut host), ["echo"]);
    let said = log.lock().unwrap().join("\n");
    assert!(
        said.contains("mcp fake tools changed \u{b7} 1 tool"),
        "{said}"
    );
}

/// A call that fails drops its connection; a notice heard while waiting for
/// it used to go with the client, leaving the reconnect's list stale.
#[test]
fn a_notice_heard_during_a_failing_call_outlives_the_connection() {
    let err = r#"{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"boom"}],"isError":true}}"#;
    let mut host = server("dropped", &[&[PROBE], &[INIT], &[ECHO], &[CHANGED, err]]);
    let log = notes(&mut host);
    assert_eq!(names(&mut host), ["echo"]);
    assert!(host.call("fake", "echo", "").is_err());
    assert_eq!(names(&mut host), ["echo"]);
    let said = log.lock().unwrap().join("\n");
    assert!(
        said.contains("mcp fake tools changed"),
        "relisted after the reconnect: {said}"
    );
}

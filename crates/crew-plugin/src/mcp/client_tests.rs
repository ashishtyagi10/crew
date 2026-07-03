#![cfg(unix)]
use std::collections::BTreeMap;

use super::*;
use crate::mcp::{McpHost, ServerConfig};

/// A canned server: replays `responses` (one JSON-RPC line each) then holds
/// its pipes open, standing in for a real MCP server.
fn canned(tag: &str, responses: &[&str]) -> ServerConfig {
    let path = std::env::temp_dir().join(format!("crew-mcp-{tag}-{}.jsonl", std::process::id()));
    std::fs::write(&path, responses.join("\n")).unwrap();
    ServerConfig {
        command: "sh".into(),
        args: vec!["-c".into(), format!("cat '{}'; sleep 5", path.display())],
        env: BTreeMap::new(),
    }
}

const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{},"serverInfo":{"name":"fake"}}}"#;
const TOOLS: &str = r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo","description":"Echo text back.\nSecond line ignored.","inputSchema":{}}]}}"#;
const CALL_OK: &str = r#"{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"hello"},{"type":"text","text":"world"}],"isError":false}}"#;
const CALL_ERR: &str = r#"{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"boom"}],"isError":true}}"#;

#[test]
fn connect_lists_tools_and_calls_one() {
    let cfg = canned("ok", &[INIT, TOOLS, CALL_OK]);
    let mut c = McpClient::connect(&cfg).unwrap();
    let tools = c.tools().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].0, "echo");
    assert_eq!(tools[0].1, "Echo text back."); // first line only
    let out = c.call("echo", serde_json::json!({"text": "hi"})).unwrap();
    assert_eq!(out, "hello\nworld");
}

#[test]
fn an_is_error_result_becomes_err() {
    let cfg = canned("err", &[INIT, TOOLS, CALL_ERR]);
    let mut c = McpClient::connect(&cfg).unwrap();
    c.tools().unwrap();
    let e = c.call("echo", serde_json::json!({})).unwrap_err();
    assert_eq!(e, "boom");
}

#[test]
fn connect_fails_cleanly_when_the_command_is_missing() {
    let cfg = ServerConfig {
        command: "no-such-mcp-server-xyz".into(),
        args: vec![],
        env: BTreeMap::new(),
    };
    assert!(McpClient::connect(&cfg)
        .unwrap_err()
        .contains("failed to launch"));
}

#[test]
fn host_lists_tools_calls_and_reports() {
    let mut servers = BTreeMap::new();
    servers.insert("fake".to_string(), canned("host", &[INIT, TOOLS, CALL_OK]));
    let mut host = McpHost::new(servers);
    assert!(!host.is_empty());
    let tools = host.tools();
    assert_eq!(tools.len(), 1);
    assert_eq!(
        (tools[0].server.as_str(), tools[0].name.as_str()),
        ("fake", "echo")
    );
    // The tool list is cached — a second read costs no request.
    assert_eq!(host.tools().len(), 1);
    assert_eq!(
        host.call("fake", "echo", r#"{"text":"hi"}"#).unwrap(),
        "hello\nworld"
    );
    let report = host.report();
    assert!(
        report.contains("fake") && report.contains("echo"),
        "got: {report}"
    );
}

#[test]
fn host_rejects_unknown_servers_and_bad_args() {
    let mut host = McpHost::new(BTreeMap::new());
    assert!(host.is_empty());
    assert!(host
        .call("ghost", "t", "{}")
        .unwrap_err()
        .contains("unknown MCP server"));
    let mut servers = BTreeMap::new();
    servers.insert("fake".to_string(), canned("badargs", &[INIT]));
    let mut host = McpHost::new(servers);
    let e = host.call("fake", "echo", "{not json").unwrap_err();
    assert!(e.contains("not valid JSON"), "got: {e}");
}

const TOOLS2: &str = r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo2","description":"Echo v2.","inputSchema":{}}]}}"#;

#[test]
fn sync_to_keeps_an_unchanged_servers_client_and_cache() {
    let mut servers = BTreeMap::new();
    servers.insert("fake".to_string(), canned("keep", &[INIT, TOOLS, CALL_OK]));
    let mut host = McpHost::new(servers.clone());
    assert_eq!(host.tools().len(), 1);
    host.sync_to(servers);
    // The original connection survived: its canned stream is already past
    // INIT and TOOLS, so the call's request id (3) matches CALL_OK. A
    // reconnect would misalign the ids and never see "hello\nworld".
    assert_eq!(host.call("fake", "echo", "{}").unwrap(), "hello\nworld");
}

#[test]
fn sync_to_swaps_a_changed_servers_client_and_cache() {
    let mut old = BTreeMap::new();
    old.insert("fake".to_string(), canned("swap-a", &[INIT, TOOLS]));
    let mut host = McpHost::new(old);
    assert_eq!(host.tools()[0].name, "echo");
    let mut new = BTreeMap::new();
    new.insert("fake".to_string(), canned("swap-b", &[INIT, TOOLS2]));
    host.sync_to(new);
    assert_eq!(host.tools()[0].name, "echo2");
}

#[test]
fn sync_to_drops_a_removed_server() {
    let mut servers = BTreeMap::new();
    servers.insert("fake".to_string(), canned("rm", &[INIT, TOOLS]));
    let mut host = McpHost::new(servers);
    assert_eq!(host.tools().len(), 1);
    host.sync_to(BTreeMap::new());
    assert!(host.is_empty());
    assert!(host.tools().is_empty());
    assert!(host
        .call("fake", "echo", "{}")
        .unwrap_err()
        .contains("unknown MCP server"));
}

#[test]
fn reload_reconnects_and_relists_every_server() {
    // The config launches `cat <path>`; rewriting <path> after the first
    // connect stands in for a running server whose tool set changed.
    let path = std::env::temp_dir().join(format!("crew-mcp-reload-{}.jsonl", std::process::id()));
    std::fs::write(&path, [INIT, TOOLS].join("\n")).unwrap();
    let cfg = ServerConfig {
        command: "sh".into(),
        args: vec!["-c".into(), format!("cat '{}'; sleep 5", path.display())],
        env: BTreeMap::new(),
    };
    let mut servers = BTreeMap::new();
    servers.insert("fake".to_string(), cfg);
    let mut host = McpHost::new(servers);
    assert_eq!(host.tools()[0].name, "echo");
    std::fs::write(&path, [INIT, TOOLS2].join("\n")).unwrap();
    assert_eq!(host.tools()[0].name, "echo", "cached until a reload");
    assert_eq!(host.reload(), vec!["fake".to_string()]);
    assert_eq!(host.tools()[0].name, "echo2");
}

#[test]
fn explicit_hosts_never_auto_sync_from_disk() {
    // `new()` (tests) pins the map; only `from_config()` tracks mcp.json —
    // if any use auto-synced, "fake" would vanish on machines without it.
    let mut servers = BTreeMap::new();
    servers.insert("fake".to_string(), canned("pin", &[INIT, TOOLS]));
    let mut host = McpHost::new(servers);
    assert_eq!(host.tools().len(), 1);
    assert!(!host.is_empty());
    let report = host.report();
    assert!(report.contains("fake"), "got: {report}");
}

#[test]
fn host_report_names_a_server_that_fails_to_launch() {
    let mut servers = BTreeMap::new();
    servers.insert(
        "broken".to_string(),
        ServerConfig {
            command: "no-such-mcp-server-xyz".into(),
            args: vec![],
            env: BTreeMap::new(),
        },
    );
    let mut host = McpHost::new(servers);
    assert!(host.tools().is_empty());
    let report = host.report();
    assert!(
        report.contains("broken") && report.contains("error"),
        "got: {report}"
    );
}

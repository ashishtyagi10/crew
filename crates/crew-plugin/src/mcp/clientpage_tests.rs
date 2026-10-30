//! A server that pages its tools (`nextCursor`) has every page read.
#![cfg(unix)]
use std::collections::BTreeMap;

use super::McpClient;
use crate::mcp::ServerConfig;

fn canned(responses: &[&str]) -> ServerConfig {
    let path = std::env::temp_dir().join(format!("crew-mcp-pages-{}.jsonl", std::process::id()));
    std::fs::write(&path, responses.join("\n")).unwrap();
    ServerConfig {
        command: "sh".into(),
        args: vec!["-c".into(), format!("cat '{}'; sleep 5", path.display())],
        env: BTreeMap::new(),
    }
}

#[test]
fn every_page_of_tools_is_read() {
    let init = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{},"serverInfo":{"name":"fake"}}}"#;
    let one = r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"search","inputSchema":{}}],"nextCursor":"p2"}}"#;
    let two = r#"{"jsonrpc":"2.0","id":3,"result":{"tools":[{"name":"fetch","inputSchema":{}}]}}"#;
    let mut c = McpClient::connect(&canned(&[init, one, two])).unwrap();
    let names: Vec<String> = c.tools().unwrap().into_iter().map(|t| t.0).collect();
    assert_eq!(names, ["search", "fetch"]);
}

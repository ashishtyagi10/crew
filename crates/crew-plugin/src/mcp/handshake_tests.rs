//! The handshake against a fake stdio server that RECORDS what crew sent and
//! answers by method, so each era's wire can be asserted line by line.
#![cfg(unix)]
use std::collections::BTreeMap;
use std::path::PathBuf;

use super::McpClient;
use crate::mcp::{McpHost, ServerConfig};

/// A server whose reply to each method is the file of that name in its
/// directory (`/` spelled `-`, `@ID@` replaced by the request's id). A
/// method with no file gets silence. Every line crew writes lands in `log`.
/// With `die-first` present, the first launch exits without reading.
fn fake(tag: &str, replies: &[(&str, &str)]) -> (ServerConfig, PathBuf) {
    let dir = std::env::temp_dir().join(format!("crew-mcp-hs-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (method, body) in replies {
        std::fs::write(dir.join(method), format!("{body}\n")).unwrap();
    }
    let script = r#"d="$1"
if [ -f "$d/die-first" ] && [ ! -f "$d/died" ]; then touch "$d/died"; exit 0; fi
while IFS= read -r line; do
  printf '%s\n' "$line" >> "$d/log"
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
  m=$(printf '%s' "$line" | sed -n 's/.*"method":"\([^"]*\)".*/\1/p' | tr / -)
  if [ -n "$id" ] && [ -f "$d/$m" ]; then sed "s/@ID@/$id/" "$d/$m"; fi
done"#;
    let cfg = ServerConfig {
        command: "sh".into(),
        args: vec![
            "-c".into(),
            script.into(),
            "fake".into(),
            dir.display().to_string(),
        ],
        env: BTreeMap::new(),
    };
    (cfg, dir.join("log"))
}

fn sent(log: &PathBuf) -> Vec<String> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

const DISCOVER: &str = r#"{"jsonrpc":"2.0","id":@ID@,"result":{"resultType":"complete","supportedVersions":["2026-07-28","2025-11-25"],"capabilities":{"tools":{}}}}"#;
const NO_METHOD: &str =
    r#"{"jsonrpc":"2.0","id":@ID@,"error":{"code":-32601,"message":"Method not found"}}"#;
const TOOLS: &str = r#"{"jsonrpc":"2.0","id":@ID@,"result":{"resultType":"complete","tools":[{"name":"echo","inputSchema":{}}],"ttlMs":1000,"cacheScope":"private"}}"#;

fn init(version: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":@ID@,"result":{{"protocolVersion":"{version}","capabilities":{{}}}}}}"#
    )
}

#[test]
fn a_modern_server_is_spoken_to_with_meta_on_every_request_and_no_handshake() {
    let (cfg, log) = fake(
        "modern",
        &[("server-discover", DISCOVER), ("tools-list", TOOLS)],
    );
    let mut c = McpClient::connect(&cfg).unwrap();
    assert_eq!(c.protocol_version(), "2026-07-28");
    assert_eq!(c.tools().unwrap()[0].0, "echo");
    let lines = sent(&log);
    assert_eq!(lines.len(), 2, "{lines:#?}");
    assert!(
        lines[0].contains(r#""method":"server/discover""#),
        "{}",
        lines[0]
    );
    assert!(
        lines.iter().all(|l| !l.contains("initialize")),
        "{lines:#?}"
    );
    for l in &lines {
        assert!(
            l.contains(r#""io.modelcontextprotocol/protocolVersion":"2026-07-28""#)
                && l.contains(r#""io.modelcontextprotocol/clientCapabilities":{}"#),
            "a modern request without its _meta: {l}"
        );
    }
}

#[test]
fn a_legacy_server_gets_initialize_and_the_old_wire() {
    let ini = init("2025-06-18");
    let (cfg, log) = fake(
        "legacy",
        &[
            ("server-discover", NO_METHOD),
            ("initialize", &ini),
            ("tools-list", TOOLS),
        ],
    );
    let mut c = McpClient::connect(&cfg).unwrap();
    assert_eq!(c.protocol_version(), "2025-06-18");
    c.tools().unwrap();
    let lines = sent(&log);
    assert!(
        lines[1].contains(r#""protocolVersion":"2025-11-25""#),
        "{}",
        lines[1]
    );
    assert!(
        lines[2].contains("notifications/initialized"),
        "{}",
        lines[2]
    );
    assert!(
        !lines[3].contains("_meta"),
        "legacy requests carry no _meta: {}",
        lines[3]
    );
}

#[test]
fn a_legacy_answer_crew_does_not_speak_disconnects_and_lists_nothing() {
    let ini = init("1999-01-01");
    let replies = [
        ("server-discover", NO_METHOD),
        ("initialize", ini.as_str()),
        ("tools-list", TOOLS),
    ];
    let (cfg, _) = fake("unknown", &replies);
    let e = McpClient::connect(&cfg).unwrap_err();
    assert!(e.contains("1999-01-01") && e.contains("2025-11-25"), "{e}");
    let (cfg, log) = fake("unknown-host", &replies);
    let mut host = McpHost::new(BTreeMap::from([("old".to_string(), cfg)]));
    assert!(host.tools().is_empty());
    assert!(sent(&log).iter().all(|l| !l.contains("tools/list")));
}

#[test]
fn a_modern_server_with_no_common_version_never_gets_initialize() {
    let refuse = r#"{"jsonrpc":"2.0","id":@ID@,"error":{"code":-32022,"message":"Unsupported protocol version","data":{"supported":["2099-01-01"],"requested":"2026-07-28"}}}"#;
    let (cfg, log) = fake(
        "refused",
        &[
            ("server-discover", refuse),
            ("initialize", &init("2025-11-25")),
        ],
    );
    let e = McpClient::connect(&cfg).unwrap_err();
    assert!(e.contains("2099-01-01") && e.contains("2026-07-28"), "{e}");
    assert!(sent(&log).iter().all(|l| !l.contains("initialize")));
}

#[test]
fn a_silent_or_dying_probe_falls_back_to_initialize() {
    let ini = init("2024-11-05");
    let (cfg, _) = fake("silent", &[("initialize", &ini)]);
    assert_eq!(
        McpClient::connect(&cfg).unwrap().protocol_version(),
        "2024-11-05"
    );
    // ...and the relaunched server answers as a one-element JSON-RPC batch,
    // which 2025-03-26 requires a client to accept.
    let batch = format!("[{ini}]");
    let (cfg, log) = fake("dies", &[("die-first", ""), ("initialize", &batch)]);
    assert_eq!(
        McpClient::connect(&cfg).unwrap().protocol_version(),
        "2024-11-05"
    );
    assert!(sent(&log)[0].contains(r#""method":"initialize""#));
}

#[test]
fn a_timed_out_request_is_cancelled_but_initialize_never_is() {
    let (cfg, log) = fake("cancel", &[("server-discover", DISCOVER)]);
    let mut c = McpClient::connect(&cfg).unwrap();
    let short = std::time::Duration::from_millis(300);
    let e = c
        .request_within("tools/list", serde_json::json!({}), short)
        .unwrap_err();
    assert!(e.contains("no response"), "{e}");
    let e = c
        .request_within("initialize", serde_json::json!({}), short)
        .unwrap_err();
    assert!(e.contains("no response"), "{e}");
    std::thread::sleep(short);
    let cancels: Vec<String> = sent(&log)
        .into_iter()
        .filter(|l| l.contains("notifications/cancelled"))
        .collect();
    assert_eq!(cancels.len(), 1, "{cancels:#?}");
    assert!(cancels[0].contains(r#""requestId":1"#), "{}", cancels[0]);
}

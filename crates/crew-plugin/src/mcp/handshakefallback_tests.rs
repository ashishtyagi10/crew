//! Legacy servers the `server/discover` probe must not lose: ones that die of
//! it after answering, and bridges that answer it with an empty result.
#![cfg(unix)]
use super::tests::{fake, init, sent, NO_METHOD, TOOLS};
use super::McpClient;

#[test]
fn a_server_that_answers_the_probe_then_exits_is_relaunched_straight_to_initialize() {
    let ini = init("2025-06-18");
    let (cfg, log) = fake(
        "quits",
        &[
            ("quit-after-probe", ""),
            ("server-discover", NO_METHOD),
            ("initialize", &ini),
            ("tools-list", TOOLS),
        ],
    );
    let mut c = McpClient::connect(&cfg).unwrap();
    assert_eq!(c.protocol_version(), "2025-06-18");
    assert_eq!(c.tools().unwrap()[0].0, "echo");
    let lines = sent(&log);
    let probes = lines.iter().filter(|l| l.contains("server/discover"));
    assert_eq!(probes.count(), 1, "probed again: {lines:#?}");
    assert!(lines[1].contains(r#""method":"initialize""#), "{lines:#?}");
}

#[test]
fn a_probe_answered_without_a_version_list_is_a_legacy_server() {
    let empty = r#"{"jsonrpc":"2.0","id":@ID@,"result":{}}"#;
    let null = r#"{"jsonrpc":"2.0","id":@ID@,"result":null}"#;
    for (tag, discover) in [("bridge-empty", empty), ("bridge-null", null)] {
        // A bridge that answers `initialize` with `{}` too: no version named.
        let (cfg, log) = fake(
            tag,
            &[
                ("server-discover", discover),
                ("initialize", empty),
                ("tools-list", TOOLS),
            ],
        );
        let mut c = McpClient::connect(&cfg).unwrap();
        assert_eq!(c.protocol_version(), "2025-11-25");
        assert_eq!(c.tools().unwrap()[0].0, "echo");
        assert!(sent(&log)[1].contains(r#""method":"initialize""#), "{tag}");
    }
}

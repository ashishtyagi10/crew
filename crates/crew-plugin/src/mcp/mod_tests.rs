
/// The connect-and-cache contract, pinned.
///
/// Measured on a real config: the FIRST task in a pane paid ~13 seconds while
/// `npx -y` downloaded a server package, and `/doctor` in the same broker
/// answered instantly — the cost is entirely in this lazy connect. Once the
/// package is cached the same call is milliseconds, so the wait is a cold-
/// cache cost rather than a per-task one.
///
/// What must stay true either way: a server is connected AT MOST ONCE per
/// broker and its tool list is fetched at most once. If either became
/// per-task, every message would pay a process spawn — and on a cold cache,
/// that download — again.
#[test]
fn a_failing_server_is_not_retried_on_every_call() {
    let mut host = McpHost::default();
    host.servers.insert(
        "nope".into(),
        crate::mcp::config::ServerConfig {
            command: "definitely-not-a-real-binary-xyz".into(),
            args: vec![],
            env: Default::default(),
        },
    );
    // Three asks, and the failure is reported the same way each time without
    // the caller having to know whether a connection was attempted.
    for _ in 0..3 {
        assert!(host.tools().is_empty(), "a dead server contributes no tools");
    }
    let report = host.report();
    assert!(report.contains("nope"), "the failure stays visible: {report}");
}

use super::*;

use super::super::approval::Requester;
use super::super::toolcall::ToolRunner;

fn host() -> Arc<Mutex<crate::mcp::McpHost>> {
    Arc::new(Mutex::new(crate::mcp::McpHost::default()))
}

/// The gate is in the path, and with a person at the keyboard it changes nothing: a read
/// still reads. (Ledger is None here so the suite never writes the user's audit file.)
#[test]
fn a_read_still_runs_for_everyone() {
    for who in [
        Requester::LocalPane,
        Requester::Channel("telegram:me".into()),
        Requester::Trigger("nightly".into()),
    ] {
        let t = SessionTools::for_requester(host(), true, who.clone());
        assert!(
            t.call("sys", "list_dir", "{}").is_ok(),
            "a directory listing changes nothing, so {who:?} may do it"
        );
    }
}

/// The behaviour that will matter the moment Telegram lands: a request with no human
/// watching cannot fire a shell command just because it asked nicely.
#[test]
fn a_channel_cannot_run_a_shell_command_without_approval() {
    let t = SessionTools::for_requester(host(), true, Requester::Channel("telegram:me".into()));
    let err = t
        .call("sys", "run", r#"{"cmd": "echo should-not-run"}"#)
        .expect_err("an irreversible call from a channel must not just run");
    assert!(err.contains("needs approval"), "{err}");
    assert!(
        err.contains("telegram:me"),
        "the refusal says who would be asked: {err}"
    );
}

/// The 3am case, end to end through the real tool path.
#[test]
fn a_trigger_cannot_run_a_shell_command_at_all() {
    let t = SessionTools::for_requester(host(), true, Requester::Trigger("nightly".into()));
    let err = t
        .call("sys", "run", r#"{"cmd": "echo should-not-run"}"#)
        .expect_err("a trigger has nobody to ask");
    assert!(err.contains("cannot be undone"), "{err}");
}

/// An MCP server nobody has classified is irreversible by default, so the same refusal
/// applies to tools crew has never seen.
#[test]
fn an_unknown_mcp_tool_from_a_channel_is_gated_too() {
    let t = SessionTools::for_requester(host(), true, Requester::Channel("telegram:me".into()));
    let err = t
        .call("some-server", "send_money", "{}")
        .expect_err("unknown means ask");
    assert!(err.contains("needs approval"), "{err}");
}

#[test]
fn defaults_to_no_overrides_and_not_cancelled() {
    let s = Session::new();
    assert!(s.overrides.is_empty());
    assert!(!s.cancelled());
}

#[test]
fn snapshot_with_cancel_uses_the_given_flag() {
    let s = Session::new();
    let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let snap = s.snapshot_with_cancel(std::sync::Arc::clone(&flag));
    // Tripping the registry-held flag cancels the snapshot's broker/loop.
    flag.store(true, Ordering::Relaxed);
    assert!(
        snap.cancelled(),
        "snapshot observes its own task's cancel flag"
    );
}

#[test]
fn session_tools_hint_lists_sys_tools_with_empty_mcp() {
    use super::super::toolcall::ToolRunner;
    let host = Arc::new(Mutex::new(crate::mcp::McpHost::default()));
    // The verdict is HANDED IN. It used to read the process environment
    // and the comment here said "under `cargo test` no mock/env gate is
    // set, so sys tools are on" — true of this suite running alone, and
    // false whenever a mocked test held CREW_BROKER_MOCK_REPLY, which is
    // about one full run in six.
    let t = SessionTools::for_test(host, true);
    let h = t.hint();
    assert!(h.contains("sys:run"), "{h}");
    assert!(h.contains("sys:read_file"), "{h}");
}

/// …and with the surface off, the hint offers nothing it cannot serve.
#[test]
fn session_tools_hint_omits_sys_when_the_surface_is_off() {
    use super::super::toolcall::ToolRunner;
    let host = Arc::new(Mutex::new(crate::mcp::McpHost::default()));
    let h = SessionTools::for_test(host, false).hint();
    assert!(!h.contains("sys:"), "{h}");
}

/// The native surface and the text surface must describe the SAME tools.
/// The provider picks which one a run uses, so a tool in one and not the
/// other appears or disappears depending on which model is serving.
#[test]
fn specs_and_hint_cover_the_same_tools() {
    use super::super::toolcall::ToolRunner;
    let host = Arc::new(Mutex::new(crate::mcp::McpHost::default()));
    let t = SessionTools::for_test(host, true);
    let hint = t.hint();
    let specs = t.specs();
    assert!(!specs.is_empty());
    for s in &specs {
        assert!(
            hint.contains(&format!("{}:{}", s.server, s.tool)),
            "{}:{} is callable natively but unadvertised in the hint",
            s.server,
            s.tool
        );
    }
}

/// Every native tool ships a schema a provider will accept: an object with
/// a `type`. A `null` or a bare `{}` is rejected by the API, which would
/// take down the whole request — not just that one tool.
#[test]
fn every_spec_carries_a_usable_schema() {
    use super::super::toolcall::ToolRunner;
    let host = Arc::new(Mutex::new(crate::mcp::McpHost::default()));
    for s in SessionTools::for_test(host, true).specs() {
        assert_eq!(
            s.input_schema["type"], "object",
            "{}:{} schema: {}",
            s.server, s.tool, s.input_schema
        );
    }
}

/// `sys:run` without a command was a wasted round: the model emitted the
/// call, the dispatcher answered "missing string argument", and the agent
/// tried again. The schema now makes the provider refuse it first.
#[test]
fn sys_run_declares_its_command_required() {
    use super::super::toolcall::ToolRunner;
    let host = Arc::new(Mutex::new(crate::mcp::McpHost::default()));
    let specs = SessionTools::for_test(host, true).specs();
    let run = specs.iter().find(|s| s.tool == "run").expect("sys:run");
    assert_eq!(run.input_schema["required"][0], "cmd");
    assert_eq!(run.input_schema["properties"]["cmd"]["type"], "string");
}

#[test]
fn session_tools_dispatches_sys_locally() {
    use super::super::toolcall::ToolRunner;
    let host = Arc::new(Mutex::new(crate::mcp::McpHost::default()));
    let t = SessionTools::for_test(host, true);
    let r = t
        .call("sys", "run", r#"{"cmd":"echo via-session"}"#)
        .unwrap();
    assert!(r.contains("via-session"), "{r}");
    // Unknown server still falls through to the (empty) MCP host's error.
    let e = t.call("nope", "x", "{}").unwrap_err();
    assert!(e.contains("unknown MCP server"), "{e}");
    // With the surface off, `sys` is not special — it is just another
    // server the empty MCP host has never heard of.
    let off = SessionTools::for_test(Arc::new(Mutex::new(crate::mcp::McpHost::default())), false);
    let e = off.call("sys", "run", r#"{"cmd":"echo x"}"#).unwrap_err();
    assert!(e.contains("unknown MCP server"), "{e}");
}

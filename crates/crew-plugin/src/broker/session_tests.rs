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

/// Retrieval, through the real tool surface rather than through the picker alone: what the
/// session shows a task, and the door out of what it hid.
mod retrieval {
    use super::*;

    /// The `sys` surface alone is well under the budget, so nothing is filtered and the hint is
    /// the whole catalog — the behaviour every crew has today.
    #[test]
    fn a_crew_with_only_its_own_tools_shows_all_of_them() {
        let t = SessionTools::for_test(host(), true);
        let hint = t.hint_for("anything at all");
        for name in ["sys:run", "sys:read_file", "sys:write_file", "sys:list_dir"] {
            assert!(hint.contains(name), "{name} missing from: {hint}");
        }
        assert!(
            !hint.contains("more tool(s) are connected"),
            "nothing was hidden, so nothing is admitted: {hint}"
        );
        assert_eq!(
            t.hint_for("anything at all"),
            t.hint(),
            "below the budget the two are the same string"
        );
    }

    /// The picker and the native path must choose alike, or a tool appears and disappears
    /// depending on which model is serving.
    #[test]
    fn the_prose_and_the_schemas_describe_the_same_tools() {
        let t = SessionTools::for_test(host(), true);
        let hint = t.hint_for("read a file");
        for s in t.specs_for("read a file") {
            assert!(hint.contains(&s.label()), "{} not in the hint", s.label());
        }
    }

    /// The door: a search reaches the whole catalog, whatever the prompt showed.
    #[test]
    fn find_tools_searches_the_catalog_and_is_a_read() {
        let t = SessionTools::for_test(host(), true);
        let out = t
            .call("sys", "find_tools", r#"{"q": "file"}"#)
            .expect("a search is a read: it looks at a list crew already holds");
        assert!(out.contains("sys:read_file"), "{out}");
        assert!(out.contains("sys:write_file"), "{out}");
        assert!(
            !out.contains("sys:run"),
            "and not the ones that do not match"
        );
    }

    /// A trigger — the least-trusted requester there is — may still SEARCH, because reading a
    /// list crew already holds does nothing to anybody's machine.
    #[test]
    fn even_a_trigger_may_look_for_a_tool() {
        let t = SessionTools::for_requester(host(), true, Requester::Trigger("nightly".into()));
        assert!(t.call("sys", "find_tools", r#"{"q": "file"}"#).is_ok());
    }

    /// A malformed call says what there is rather than erroring: the model's next move is to
    /// search again with a word in it.
    #[test]
    fn a_search_with_no_query_lists_nothing_and_says_how_many_there_are() {
        let t = SessionTools::for_test(host(), true);
        let out = t.call("sys", "find_tools", "not json").expect("no error");
        assert!(out.contains("no tool matches"), "{out}");
    }
}

/// Integrations: a manifest is a tool surface, with no Rust and no restart.
mod integrations {
    use super::*;
    use crate::broker::integration;

    const WEATHER: &str = r#"{
      "name": "weather",
      "base_url": "https://api.example.com",
      "auth": {"kind": "bearer", "env": "CREW_TEST_WEATHER_TOKEN"},
      "tools": [
        {"name": "forecast", "description": "the forecast for a city",
         "path": "/f/{city}", "tier": "read"},
        {"name": "subscribe", "description": "sign up for alerts", "method": "POST",
         "path": "/sub"}
      ]
    }"#;

    fn tools(sys: bool) -> SessionTools {
        SessionTools::with_integrations(sys, vec![integration::parse(WEATHER).unwrap()])
    }

    /// The whole contract in one assertion: a file appeared, and the agents can see its tools.
    #[test]
    fn a_manifest_puts_its_tools_on_the_same_surface_as_everything_else() {
        let t = tools(true);
        let hint = t.hint_for("what is the weather in Oslo");
        assert!(hint.contains("weather:forecast"), "{hint}");
        assert!(hint.contains("the forecast for a city"), "{hint}");
        let specs: Vec<String> = t
            .specs_for("what is the weather in Oslo")
            .iter()
            .map(|s| s.label())
            .collect();
        assert!(specs.contains(&"weather:forecast".to_string()), "{specs:?}");
    }

    /// The manifest's own tier reaches the gate — which is the difference between an
    /// integration that can read the weather and one that can subscribe you to something.
    #[test]
    fn the_manifests_tier_is_the_one_the_gate_uses() {
        let t = tools(true);
        assert_eq!(
            t.tier_for("weather", "forecast"),
            crate::broker::tier::Tier::Read
        );
        assert_eq!(
            t.tier_for("weather", "subscribe"),
            crate::broker::tier::Tier::Irreversible,
            "a tool whose manifest says nothing must ask"
        );
    }

    /// A trigger firing at 3am may read the forecast and may not sign anybody up for anything.
    #[test]
    fn a_scheduled_run_is_held_to_the_manifests_tiers() {
        let t = SessionTools {
            requester: Requester::Trigger("nightly".into()),
            ledger: None,
            ..tools(true)
        };
        let err = t
            .call("weather", "subscribe", "{}")
            .expect_err("irreversible, and nobody is awake to ask");
        assert!(err.contains("cannot be undone"), "{err}");
    }

    /// The credential is checked before anything is sent, and the message names the variable
    /// rather than coming back as somebody else's 401.
    #[test]
    fn a_missing_credential_says_which_variable_to_set_without_a_round_trip() {
        std::env::remove_var("CREW_TEST_WEATHER_TOKEN");
        let err = tools(true)
            .call("weather", "forecast", r#"{"city": "Oslo"}"#)
            .expect_err("no token, no call");
        assert!(err.contains("CREW_TEST_WEATHER_TOKEN"), "{err}");
    }

    /// A tool the manifest does not declare is refused by name, not attempted.
    #[test]
    fn a_tool_the_manifest_never_declared_is_refused() {
        let err = tools(true)
            .call("weather", "delete_everything", "{}")
            .expect_err("not in the manifest");
        assert!(err.contains("no tool"), "{err}");
    }
}

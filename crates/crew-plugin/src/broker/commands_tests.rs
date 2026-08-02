use super::*;

fn run(text: &str) -> Vec<PluginEvent> {
    let mut session = Session::new();
    let mut out = Vec::new();
    handle(
        &mut session,
        text,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            out.push(ev);
            Ok(())
        },
    )
    .unwrap();
    out
}

fn text_of(ev: &PluginEvent) -> &str {
    match ev {
        PluginEvent::Message { text, .. } => text,
        _ => "",
    }
}

#[test]
fn detects_commands() {
    assert!(is_command("/help"));
    assert!(is_command("  /model"));
    assert!(!is_command("do the thing"));
    assert!(!is_command("@planner go"));
}

#[test]
fn quick_commands_answer_inline_but_constructs_do_not() {
    // The retired `/fan` and `/loop` are quick now — see `retire_tests`.
    for quick in ["/help", "/model coder x", "/status", "/diff", "/nonsense"] {
        assert!(is_quick(quick), "{quick}");
    }
    for long in ["/goal ship it", "a plain task"] {
        assert!(!is_quick(long), "{long}");
    }
}

#[test]
fn help_lists_constructs() {
    let evs = run("/help");
    assert_eq!(evs.len(), 1);
    let t = text_of(&evs[0]);
    assert!(t.contains("/model"), "{t}");
    // The roster construct was folded into `/model`; help must not advertise
    // a construct the router no longer answers.
    assert!(!t.contains("/agents"), "{t}");
}

#[test]
fn help_includes_the_concurrency_tip() {
    let evs = run("/help");
    let t = text_of(&evs[0]);
    assert!(t.contains("tip: tasks run in the background"), "{t}");
    assert!(t.contains("the footer lists them"), "{t}");
    assert!(t.contains("/stop #n cancels one"), "{t}");
}

#[test]
fn help_lists_the_diff_construct() {
    let evs = run("/help");
    let t = text_of(&evs[0]);
    assert!(t.contains("/diff"), "{t}");
    // What the line must TELL a reader, not which git command it runs. It
    // said "git diff --stat" — which stopped being true the moment the
    // comparison had to include files git had never been told about.
    assert!(t.contains("new files"), "{t}");
}

#[test]
fn diff_reports_something_for_the_current_repo() {
    // Read-only: exercises the real working tree — safe because /diff never
    // mutates it.
    let evs = run("/diff");
    assert_eq!(evs.len(), 1);
    assert!(!text_of(&evs[0]).is_empty());
}

#[test]
fn unknown_command_points_at_help() {
    let evs = run("/frobnicate now");
    let t = text_of(&evs[0]);
    assert!(t.contains("unknown construct /frobnicate"), "{t}");
    assert!(t.contains("/help"), "{t}");
}

#[test]
/// Bare `/model` inherited the listing role from the deleted `/agents`,
/// INCLUDING its fresh-Roster emit — that was the live-reload path for manifest
/// edits, and folding one construct into another must not silently drop it.
fn bare_model_reemits_the_roster_before_its_report() {
    // In tests no API key is guaranteed; the roster may be empty, but the
    // Roster event always precedes the report so the pane's badges refresh.
    let evs = run("/model");
    assert_eq!(evs.len(), 2);
    assert!(matches!(evs[0], PluginEvent::Roster { .. }), "{evs:?}");
    assert!(!text_of(&evs[1]).is_empty());
}

use crate::broker::testenv;

#[test]
fn model_pins_an_agent_and_reemits_the_roster() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let mut session = Session::new();
    let mut evs = Vec::new();
    handle(
        &mut session,
        "/model coder qwen-turbo",
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(session.overrides.get("coder").unwrap(), "qwen-turbo");
    // A fresh Roster event precedes the confirmation message.
    match &evs[0] {
        PluginEvent::Roster { agents } => {
            let coder = agents.iter().find(|a| a.name == "coder").unwrap();
            assert_eq!(coder.model, "qwen-turbo");
        }
        ev => panic!("expected Roster first, got {ev:?}"),
    }
    assert!(text_of(&evs[1]).contains("coder now runs qwen-turbo"));
}

#[test]
fn model_default_clears_the_pin() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let mut session = Session::new();
    session.overrides.insert("coder".into(), "x".into());
    let mut evs = Vec::new();
    handle(
        &mut session,
        "/model coder default",
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    assert!(session.overrides.is_empty());
    assert!(text_of(&evs[1]).contains("provider default"));
}

#[test]
fn model_all_pins_every_agent_and_reemits_the_roster() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let mut session = Session::new();
    let names = session.registry().names();
    assert!(names.len() >= 2, "the trio registers several agents");
    let mut evs = Vec::new();
    handle(
        &mut session,
        "/model all qwen-turbo",
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    // Every registered agent is now pinned to the picked model.
    for n in &names {
        assert_eq!(
            session.overrides.get(n).map(String::as_str),
            Some("qwen-turbo")
        );
    }
    match &evs[0] {
        PluginEvent::Roster { agents } => {
            assert!(agents.iter().all(|a| a.model == "qwen-turbo"));
        }
        ev => panic!("expected Roster first, got {ev:?}"),
    }
    assert!(text_of(&evs[1]).contains("all agents now run qwen-turbo"));
}

#[test]
fn model_all_default_clears_every_pin() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let mut session = Session::new();
    for n in session.registry().names() {
        session.overrides.insert(n, "x".into());
    }
    let mut evs = Vec::new();
    handle(
        &mut session,
        "/model all default",
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    assert!(session.overrides.is_empty(), "all pins cleared");
    assert!(text_of(&evs[1]).contains("provider default"));
}

#[test]
fn model_unknown_agent_lists_the_roster() {
    let _g = testenv::mock("ok\n@done");
    let evs = run("/model ghost qwen-max");
    assert!(text_of(&evs[0]).contains("unknown agent"), "{evs:?}");
}

#[test]
fn expand_alias_maps_known_short_forms() {
    // `/s` is gone with the construct it pointed at.
    assert_eq!(expand_alias("/s"), "/s");
    assert_eq!(expand_alias("/m coder qwen"), "/model coder qwen");
    assert_eq!(expand_alias("/h"), "/help");
    // `/a` is gone with the construct it pointed at.
    assert_eq!(expand_alias("/a"), "/a");
    // `/t` is gone with the construct it pointed at.
    assert_eq!(expand_alias("/t"), "/t");
    assert_eq!(expand_alias("/d"), "/diff");
}

#[test]
fn expand_alias_leaves_non_aliases_unchanged() {
    // Already the long form — not an alias key, so untouched.
    assert_eq!(expand_alias("/status"), "/status");
    assert_eq!(expand_alias("hello"), "hello");
    // Unknown short form — no match, unchanged.
    assert_eq!(expand_alias("/x"), "/x");
}

#[test]
fn help_documents_the_aliases() {
    let evs = run("/help");
    let t = text_of(&evs[0]);
    assert!(t.contains("aliases: /h /d /m /r"), "{t}");
}

#[test]
fn closest_construct_suggests_near_typos() {
    assert_eq!(closest_construct("stauts"), Some("standup"));
    assert_eq!(closest_construct("hlep"), Some("help"));
    assert_eq!(closest_construct("relaod"), Some("reload"));
    assert_eq!(closest_construct("zzzzz"), None);
}

#[test]
fn reload_is_quick_and_listed_in_help() {
    assert!(is_quick("/reload"));
    let evs = run("/help");
    let t = text_of(&evs[0]);
    assert!(t.contains("/reload"), "{t}");
    assert!(t.contains("without a restart"), "{t}");
}

#[test]
fn expand_alias_maps_r_to_reload() {
    assert_eq!(expand_alias("/r"), "/reload");
}

#[test]
fn reload_reemits_the_roster_and_reports_every_surface() {
    let _g = testenv::mock("ok\n@done");
    let mut session = Session::new();
    let mut evs = Vec::new();
    handle(
        &mut session,
        "/reload",
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    assert!(matches!(evs[0], PluginEvent::Roster { .. }), "{evs:?}");
    let t = text_of(&evs[1]);
    assert!(t.contains("skills:"), "{t}");
    assert!(t.contains("plugin agents:"), "{t}");
    assert!(t.contains("mcp:"), "{t}");
}

#[test]
fn unknown_construct_suggests_the_closest_match() {
    let evs = run("/dcotor");
    let t = text_of(&evs[0]);
    assert!(t.contains("did you mean /doctor"), "{t}");
}

/// `/help` is the THIRD copy of the construct list — the router's `CONSTRUCTS`
/// and the host's palette are the other two — and it is the only one a
/// stdio host ever sees. A construct missing from it is invisible to anyone
/// driving the broker without a palette, which is exactly how `/reload`
/// stayed hidden for eleven releases on the host side.
#[test]
fn help_documents_every_construct_the_router_answers() {
    let help = super::HELP;
    for c in super::broker_constructs() {
        assert!(
            help.contains(&format!("/{c}")),
            "/{c} routes but /help never mentions it"
        );
    }
}

/// …and does not advertise one that no longer routes. Every `/name` in the
/// help text must be answerable, ignoring the aliases line.
#[test]
fn help_advertises_nothing_that_routes_nowhere() {
    for line in super::HELP.lines() {
        if line.trim_start().starts_with("aliases:") {
            continue;
        }
        let Some(rest) = line.trim_start().strip_prefix('/') else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        if name.is_empty() {
            continue;
        }
        assert!(
            super::broker_constructs().contains(&name.as_str()),
            "/help advertises /{name}, which routes nowhere"
        );
    }
}

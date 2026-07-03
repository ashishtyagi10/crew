use super::*;

fn run(text: &str) -> Vec<PluginEvent> {
    let mut session = Session::new();
    let mut out = Vec::new();
    handle(&mut session, text, &mut |ev| {
        out.push(ev);
        Ok(())
    })
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
    assert!(is_command("  /agents"));
    assert!(!is_command("do the thing"));
    assert!(!is_command("@planner go"));
}

#[test]
fn quick_commands_answer_inline_but_constructs_do_not() {
    for quick in [
        "/help",
        "/agents",
        "/model coder x",
        "/status",
        "/diff",
        "/nonsense",
    ] {
        assert!(is_quick(quick), "{quick}");
    }
    for long in [
        "/fan build it",
        "/loop 3 x",
        "/goal ship it",
        "a plain task",
    ] {
        assert!(!is_quick(long), "{long}");
    }
}

#[test]
fn help_lists_constructs() {
    let evs = run("/help");
    assert_eq!(evs.len(), 1);
    let t = text_of(&evs[0]);
    assert!(t.contains("/agents"), "{t}");
}

#[test]
fn help_includes_the_concurrency_tip() {
    let evs = run("/help");
    let t = text_of(&evs[0]);
    assert!(t.contains("tip: tasks run in the background"), "{t}");
    assert!(t.contains("/tasks lists them"), "{t}");
    assert!(t.contains("/stop #n cancels one"), "{t}");
}

#[test]
fn help_lists_the_diff_construct() {
    let evs = run("/help");
    let t = text_of(&evs[0]);
    assert!(t.contains("/diff"), "{t}");
    assert!(t.contains("git diff --stat"), "{t}");
}

#[test]
fn diff_reports_something_for_the_current_repo() {
    // Read-only: exercises the real cwd, like `/agents` does above — safe
    // because /diff never mutates the working tree.
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
fn agents_reports_roster_or_keys_hint() {
    // In tests no API key is guaranteed; either a roster line or the
    // no-agents hint is acceptable — both are a Message.
    let evs = run("/agents");
    assert_eq!(evs.len(), 1);
    assert!(!text_of(&evs[0]).is_empty());
}

use crate::broker::testenv;

#[test]
fn model_pins_an_agent_and_reemits_the_roster() {
    let _g = testenv::mock("ok\n@done");
    let mut session = Session::new();
    let mut evs = Vec::new();
    handle(&mut session, "/model coder qwen-turbo", &mut |ev| {
        evs.push(ev);
        Ok(())
    })
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
    let _g = testenv::mock("ok\n@done");
    let mut session = Session::new();
    session.overrides.insert("coder".into(), "x".into());
    let mut evs = Vec::new();
    handle(&mut session, "/model coder default", &mut |ev| {
        evs.push(ev);
        Ok(())
    })
    .unwrap();
    assert!(session.overrides.is_empty());
    assert!(text_of(&evs[1]).contains("provider default"));
}

#[test]
fn status_reports_totals_pins_and_running_state() {
    let _g = testenv::mock("ok\n@done");
    let mut session = Session::new();
    session
        .overrides
        .insert("coder".into(), "qwen-turbo".into());
    session.turns.store(4, std::sync::atomic::Ordering::Relaxed);
    session
        .tokens
        .store(950, std::sync::atomic::Ordering::Relaxed);
    let t = super::status_report(&session, 2);
    assert!(t.contains("2 task(s) running"), "{t}");
    assert!(t.contains("turns: 4"), "{t}");
    assert!(t.contains("~950 tok"), "{t}");
    assert!(t.contains("coder \u{2192} qwen-turbo"), "{t}");
    assert!(t.contains("planner"), "roster included: {t}");
}

#[test]
fn model_unknown_agent_lists_the_roster() {
    let _g = testenv::mock("ok\n@done");
    let evs = run("/model ghost qwen-max");
    assert!(text_of(&evs[0]).contains("unknown agent"), "{evs:?}");
}

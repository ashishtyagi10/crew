//! A prompt hook through the real `crew-broker-plugin`: a refused task never dials an agent and
//! settles (its `Task` ends), an allowed one runs as it always did. The hooks file is the
//! user-level one, so each test points the broker's config directory at its own temp dir and
//! turns `CREW_HOOKS` back on (the harness turns it off).
#![cfg(unix)]
mod common;
use common::{messages, run_broker, seed_specialists, unique_dir, PluginEvent};

const SEND: &str = r#"{"type":"send","channel":"crew","text":"@planner do task a"}"#;

/// Run one `@planner` task under `hooks`, with the config dir (macOS and XDG) in the test dir.
fn run(tag: &str, hooks: &str) -> Vec<PluginEvent> {
    let dir = unique_dir(tag);
    seed_specialists(&dir, &["planner"]);
    let home = dir.join("home");
    for rel in ["home/Library/Application Support/crew", "xdg/crew"] {
        let d = dir.join(rel);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("hooks.json"), hooks).unwrap();
    }
    let (home, xdg) = (home.to_string_lossy(), dir.join("xdg"));
    let xdg = xdg.to_string_lossy();
    let env = [
        ("CREW_BROKER_MOCK_REPLY", "did it\n@done"),
        ("CREW_HOOKS", "1"),
        ("HOME", &*home),
        ("XDG_CONFIG_HOME", &*xdg),
    ];
    run_broker(&dir, &env, &[SEND])
}

fn ended(ev: &[PluginEvent]) -> bool {
    ev.iter()
        .any(|e| matches!(e, PluginEvent::Task { running: false, .. }))
}

#[test]
fn a_refused_prompt_runs_nothing_says_why_and_settles() {
    let ev = run(
        "hook-prompt-block",
        r#"{"prompt": [{"cmd": "echo 'no push today' >&2; exit 2"}]}"#,
    );
    let msgs = messages(&ev);
    let said: Vec<_> = msgs
        .iter()
        .filter(|(_, t)| t.starts_with("blocked by hook"))
        .collect();
    assert_eq!(said.len(), 1, "{msgs:?}");
    assert_eq!(said[0].1, "blocked by hook: no push today");
    assert!(!msgs.iter().any(|(_, t)| t.contains("did it")), "{msgs:?}");
    assert!(ended(&ev), "the task never settled: {ev:?}");
}

#[test]
fn an_allowing_prompt_hook_lets_the_task_run() {
    let ev = run("hook-prompt-allow", r#"{"prompt": [{"cmd": "echo fine"}]}"#);
    let msgs = messages(&ev);
    assert!(msgs.iter().any(|(_, t)| t.contains("did it")), "{msgs:?}");
    assert!(!msgs.iter().any(|(_, t)| t.contains("blocked by hook")));
    assert!(ended(&ev));
}

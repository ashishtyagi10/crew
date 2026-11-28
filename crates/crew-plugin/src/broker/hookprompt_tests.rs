//! Prompt hooks through a real `sh`: what refuses a task, what rides along as context, and what
//! a hook that cannot run means. Every file is a temp file, never the developer's `hooks.json`.
use super::super::tests::source;
use super::*;

fn prompts(tag: &str, cmd: &str, on_failure: Option<&str>) -> Hooks {
    let mut rule = serde_json::json!({ "cmd": cmd });
    if let Some(f) = on_failure {
        rule["on_failure"] = f.into();
    }
    source(tag, &serde_json::json!({ "prompt": [rule] }).to_string()).load()
}

#[test]
fn a_prompt_hook_refuses_by_exit_2_or_by_its_json() {
    let two = prompts("p-two", "echo 'no secrets' >&2; exit 2", None);
    assert_eq!(two.prompt("hi"), Err("blocked by hook: no secrets".into()));
    let json = r#"echo '{"decision": "block", "reason": "not today"}'"#;
    let by_json = prompts("p-json", json, None);
    assert_eq!(
        by_json.prompt("hi"),
        Err("blocked by hook: not today".into())
    );
}

#[test]
fn a_prompt_hook_that_prints_adds_context_and_one_that_exits_1_does_not() {
    let said = prompts("p-ctx", "echo 'branch is dirty'", None);
    assert_eq!(said.prompt("hi"), Ok(Some("branch is dirty".into())));
    assert_eq!(prompts("p-quiet", "true", None).prompt("hi"), Ok(None));
    // Only exit 2 says no; trouble is a note for the person, and its stdout is not context.
    let trouble = prompts("p-one", "echo oops; exit 1", None);
    assert_eq!(trouble.prompt("hi"), Ok(None));
    let long = prompts("p-cap", "head -c 5000 /dev/zero | tr '\\0' x", None);
    let ctx = long.prompt("hi").unwrap().unwrap();
    assert!(ctx.ends_with("(cut at 2000 chars)"), "{ctx}");
}

#[test]
fn a_prompt_hook_reads_the_task_as_one_json_line() {
    let s = source(
        "p-stdin",
        r#"{"prompt": [{"cmd": "cat > seen"}, {"cmd": "echo second"}]}"#,
    );
    assert_eq!(s.load().prompt("fix \"it\""), Ok(Some("second".into())));
    let seen = std::fs::read_to_string(s.cwd.as_ref().unwrap().join("seen")).unwrap();
    assert_eq!(seen.lines().count(), 1);
    let v: serde_json::Value = serde_json::from_str(&seen).unwrap();
    assert_eq!(
        (v["event"].as_str(), v["prompt"].as_str()),
        (Some("prompt"), Some("fix \"it\""))
    );
    assert!(v["cwd"].as_str().is_some_and(|c| !c.is_empty()));
}

#[test]
fn a_hook_that_cannot_run_refuses_only_when_it_says_so() {
    let missing = "/no/such/prompt-hook";
    assert_eq!(prompts("p-miss", missing, None).prompt("hi"), Ok(None));
    let guard = prompts("p-guard", missing, Some("block")).prompt("hi");
    let why = guard.unwrap_err();
    assert!(
        why.starts_with("blocked by hook: ") && why.contains("set to block"),
        "{why}"
    );
}

/// The first refusal wins, and a refusal later in the list still beats context before it.
#[test]
fn a_later_hook_can_still_refuse() {
    let json = r#"{"prompt": [{"cmd": "echo ctx"}, {"cmd": "exit 2"}]}"#;
    let refused = source("p-late", json).load().prompt("hi");
    assert_eq!(
        refused,
        Err("blocked by hook: a hook said no and gave no reason".into())
    );
}

#[test]
fn the_file_takes_prompt_rules_and_refuses_a_match_on_one() {
    let bad = source("p-match", r#"{"prompt": [{"match": "*", "cmd": "true"}]}"#).load();
    assert!(bad.prompt("hi").unwrap_err().contains("match"));
    let broken = source("p-broken", r#"{"promts": []}"#).load();
    assert!(broken.prompt("hi").unwrap_err().contains("CREW_HOOKS=0"));
}

/// Off is no hooks at all, and the context reaches the model only through `with_memory`.
#[test]
fn the_switch_and_the_scope() {
    assert_eq!(gate(None, "hi"), Ok(None));
    let s = source("p-gate", r#"{"prompt": [{"cmd": "echo careful"}]}"#);
    let said = gate(Some(&s), "hi").unwrap();
    assert_eq!(block(), None);
    {
        let _in = enter(said);
        let task = crate::broker::memory::with_memory("do it", "do it");
        assert!(
            task.ends_with("From your prompt hook:\ncareful\n\ndo it"),
            "{task}"
        );
    }
    assert_eq!(block(), None);
}

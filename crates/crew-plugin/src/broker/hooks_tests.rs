//! The hook contract, end to end through a real `sh`: what blocks, what lets through, what a
//! hook reads, and what the agent is shown. Every file here is a temp file, never the
//! developer's own `hooks.json`, and every hook runs in a temp directory of its own.
use std::time::{Duration, Instant};

use super::*;

/// A hooks file holding `json`, in a directory of its own that its hooks run in.
pub(super) fn source(tag: &str, json: &str) -> Source {
    let dir = std::env::temp_dir().join(format!("crew-hooks-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("hooks.json");
    std::fs::write(&path, json).unwrap();
    Source {
        path,
        deadline: DEADLINE,
        cwd: Some(dir),
    }
}

/// A file with one rule at `stage`. Built as JSON rather than spliced into a string, so a
/// command's own quotes need no escaping by hand.
pub(super) fn one(stage: &str, pattern: &str, cmd: &str, on_failure: Option<&str>) -> String {
    let mut rule = serde_json::json!({"match": pattern, "cmd": cmd});
    if let Some(f) = on_failure {
        rule["on_failure"] = f.into();
    }
    serde_json::json!({ stage: [rule] }).to_string()
}

fn pre(tag: &str, cmd: &str, on_failure: Option<&str>) -> Hooks {
    source(tag, &one("pre_tool", "sys:run", cmd, on_failure)).load()
}

pub(super) const ARGS: &str = r#"{"cmd": "git push"}"#;

#[test]
fn a_pre_hook_exiting_2_blocks_with_its_stderr_as_the_reason() {
    let h = pre("exit2", "echo 'not on main' >&2; exit 2", None);
    assert_eq!(
        h.pre("sys:run", ARGS),
        Err("blocked by hook: not on main".into())
    );
}

#[test]
fn a_pre_hook_printing_a_block_decision_blocks_with_its_reason() {
    let h = pre(
        "json",
        r#"echo '{"decision": "block", "reason": "use -p"}'"#,
        None,
    );
    assert_eq!(
        h.pre("sys:run", ARGS),
        Err("blocked by hook: use -p".into())
    );
}

/// Exit 0 allows; so does any code but 2 — Claude Code's rule, so a hook written for it means
/// the same thing here.
#[test]
fn a_pre_hook_exiting_anything_but_2_lets_the_call_through() {
    assert_eq!(pre("exit0", "exit 0", None).pre("sys:run", ARGS), Ok(()));
    assert_eq!(
        pre("exit1", "exit 1", Some("block")).pre("sys:run", ARGS),
        Ok(())
    );
}

#[test]
fn a_hook_for_another_tool_never_runs() {
    let s = source("other", &one("pre_tool", "sys:edit", "touch ran", None));
    let h = s.load();
    assert_eq!(h.pre("sys:run", ARGS), Ok(()));
    let ran = s.cwd.as_ref().unwrap().join("ran");
    assert!(!ran.exists(), "a sys:edit hook ran for sys:run");
    assert_eq!(h.pre("sys:edit", "{}"), Ok(()));
    assert!(
        ran.exists(),
        "the matching hook runs, in the directory it was given"
    );
}

#[test]
fn a_pattern_is_an_exact_label_a_whole_server_or_everything() {
    assert!(matches("sys:run", "sys:run") && !matches("sys:run", "sys:run_all"));
    assert!(matches("sys:*", "sys:edit") && matches("*", "github:create_issue"));
    assert!(!matches("sys:*", "sysx:run") && !matches("sys:*", "github:sys"));
}

/// The command is not there: `sh` says 127, and the hook never ran — so it is a failure to
/// run, which `on_failure` decides, not an exit code that lets the call through.
#[test]
fn a_hook_that_cannot_run_blocks_only_when_it_says_so() {
    let missing = "crew-no-such-hook-anywhere";
    let err = pre("missb", missing, Some("block")).pre("sys:run", ARGS);
    let err = err.expect_err("a guard that cannot run refuses");
    assert!(
        err.starts_with(&format!("blocked by hook: `{missing}` could not run")),
        "{err}"
    );
    assert_eq!(pre("missa", missing, None).pre("sys:run", ARGS), Ok(()));
}

#[test]
fn a_hook_past_its_deadline_is_stopped_and_on_failure_decides() {
    for (tag, on_failure) in [("slowb", Some("block")), ("slowa", None)] {
        let mut s = source(tag, &one("pre_tool", "*", "sleep 10", on_failure));
        s.deadline = Duration::from_millis(300);
        let started = Instant::now();
        let verdict = s.load().pre("sys:run", ARGS);
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "the hook was waited out"
        );
        match on_failure {
            Some(_) => assert!(verdict.unwrap_err().contains("deadline")),
            None => assert_eq!(verdict, Ok(())),
        }
    }
}

/// What a hook reads: one JSON line naming the event, the tool, its arguments and where.
#[test]
fn a_pre_hook_reads_the_call_as_one_json_line_on_stdin() {
    let s = source(
        "stdin",
        &one("pre_tool", "sys:run", "cat >&2; exit 2", None),
    );
    let why = s.load().pre("sys:run", r#"{"cmd": "ls"}"#).unwrap_err();
    let line: serde_json::Value =
        serde_json::from_str(why.strip_prefix("blocked by hook: ").unwrap()).unwrap();
    assert_eq!(line["event"], "pre_tool");
    assert_eq!(line["tool"], "sys:run");
    assert_eq!(line["args"]["cmd"], "ls");
    assert_eq!(line["cwd"], s.cwd.unwrap().to_string_lossy().as_ref());
}

/// `CREW_HOOKS=0` is the switch; the env read itself is one line in `Source::live`.
#[test]
fn the_kill_switch_takes_the_file_away() {
    let config = std::path::PathBuf::from("/c");
    assert!(file::switch(None) && file::switch(Some("1")) && !file::switch(Some("0")));
    assert_eq!(file::location(false, Some(config.clone())), None);
    assert_eq!(file::location(true, None), None);
    let at = file::location(true, Some(config)).unwrap();
    assert!(at.ends_with("crew/hooks.json"), "{}", at.display());
}

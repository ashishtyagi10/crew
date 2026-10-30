//! Which run continues the pane's last Claude Code session, and what
//! happens when Claude Code refuses to.
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use super::Resume;
use crate::broker::adapter::{Adapter, HopStream};
use crate::broker::cancelscope;
use crate::broker::claudeagent::ClaudeAgent;

#[test]
fn only_the_first_run_of_a_new_message_takes_the_session_once() {
    let r = Resume::default();
    let t = |_: u8| Some(Arc::new(AtomicBool::new(false)));
    let (one, two, three, four) = (t(1), t(2), t(3), t(4));
    assert_eq!(r.take_for(&one), None, "nothing yet");
    r.keep("s1", &one);
    assert_eq!(r.take_for(&one), None, "a later dial of the same message");
    assert_eq!(r.take_for(&two).as_deref(), Some("s1"), "the next message");
    assert_eq!(
        r.take_for(&three),
        None,
        "taken: no second run continues it"
    );
    r.keep("s2", &three);
    r.forget();
    assert_eq!(r.take_for(&four), None, "a stop or a failure forgets it");
    r.keep("s3", &None);
    assert_eq!(r.take_for(&four), None, "outside a task nothing is kept");
}

/// A fake `claude` that logs its argv and refuses `--resume` when `refuse`.
fn fake(refuse: bool) -> (std::path::PathBuf, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!(
        "crew-resume-{}-{:?}-{refuse}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let log = dir.join("argv.log");
    let no = r#"{"type":"result","subtype":"error_during_execution","is_error":true,"result":"","errors":["No conversation found"]}"#;
    let ok = r#"{"type":"result","subtype":"success","is_error":false,"result":"done","session_id":"s-new","usage":{"input_tokens":1,"output_tokens":1}}"#;
    let refusal = match refuse {
        true => format!("case \"$*\" in *--resume*) echo '{no}'; exit 1;; esac\n"),
        false => String::new(),
    };
    let script = format!(
        "#!/bin/sh\necho \"$*\" >> '{}'\n{refusal}echo '{ok}'\n",
        log.display()
    );
    let (src, path) = (dir.join("claude.src"), dir.join("claude"));
    std::fs::write(&src, script).unwrap();
    std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o755)).unwrap();
    let cp = std::process::Command::new("cp")
        .arg("-p")
        .arg(&src)
        .arg(&path)
        .status();
    assert!(cp.unwrap().success());
    (path, log)
}

/// Two messages, each its own task: what the second run was started with.
fn second_message(refuse: bool) -> (Result<String, String>, String) {
    let (path, log) = fake(refuse);
    let agent = ClaudeAgent {
        program: path.to_str().unwrap().into(),
        model: None,
        idle_floor: Duration::from_secs(30),
        resume: Resume::default(),
    };
    let dial = |agent: &ClaudeAgent| {
        let _scope = cancelscope::enter(Arc::new(AtomicBool::new(false)));
        let got = agent.call_with_usage_ticked("go", Duration::from_secs(30), &HopStream::noop());
        got.map(|(t, _)| t)
    };
    dial(&agent).unwrap();
    let got = dial(&agent);
    (got, std::fs::read_to_string(log).unwrap())
}

#[test]
fn the_next_message_continues_the_session_the_last_one_ended_on() {
    let (got, argv) = second_message(false);
    assert_eq!(got.as_deref(), Ok("done"));
    let runs: Vec<&str> = argv.lines().collect();
    assert_eq!(runs.len(), 2, "{argv}");
    assert!(!runs[0].contains("--resume"), "{}", runs[0]);
    assert!(runs[1].ends_with("--resume s-new"), "{}", runs[1]);
}

#[test]
fn a_resume_claude_code_refuses_runs_again_fresh() {
    let (got, argv) = second_message(true);
    assert_eq!(got.as_deref(), Ok("done"), "the message is still answered");
    let runs: Vec<&str> = argv.lines().collect();
    assert_eq!(runs.len(), 3, "first, refused resume, fresh: {argv}");
    assert!(!runs[2].contains("--resume"));
}

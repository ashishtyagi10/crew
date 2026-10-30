//! A stop reaches an agent CLI that is still running: a fake that prints a
//! line and then sleeps far past the test is ended within the grace, on both
//! read loops, and the dial says it was stopped.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::broker::adapter::{Adapter, HopStream};
use crate::broker::cancelscope;
use crate::broker::claudeagent::ClaudeAgent;

/// A CLI that says one stream line and then works for 30 s.
fn sleeper() -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!(
        "crew-clistop-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let (src, path) = (dir.join("claude.src"), dir.join("claude"));
    let script = "#!/bin/sh\necho '{\"type\":\"system\",\"subtype\":\"status\",\"status\":\"requesting\"}'\nexec sleep 30\n";
    std::fs::write(&src, script).unwrap();
    std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o755)).unwrap();
    // Copied, not written in place: see claudeagent_tests::put (ETXTBSY).
    let cp = std::process::Command::new("cp")
        .arg("-p")
        .arg(&src)
        .arg(&path)
        .status();
    assert!(cp.unwrap().success());
    path
}

/// `dial`, with the task's stop pressed 300 ms in: what it returned, and when.
fn stopped_while<T>(dial: impl FnOnce() -> T) -> (T, Duration) {
    let flag = Arc::new(AtomicBool::new(false));
    let _scope = cancelscope::enter(Arc::clone(&flag));
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        flag.store(true, Ordering::Relaxed);
    });
    let t0 = Instant::now();
    (dial(), t0.elapsed())
}

#[test]
fn a_stop_ends_the_running_claude_relay() {
    let agent = ClaudeAgent {
        program: sleeper().to_str().unwrap().into(),
        model: None,
        idle_floor: Duration::from_secs(60),
    };
    let (got, took) = stopped_while(|| {
        agent.call_with_usage_ticked("hi", Duration::from_secs(60), &HopStream::noop())
    });
    assert_eq!(got.unwrap_err(), cancelscope::STOPPED);
    assert!(took < Duration::from_secs(5), "took {took:?}");
}

#[test]
fn a_stop_ends_a_running_agent_cli() {
    let program = sleeper();
    let (got, took) = stopped_while(|| {
        crate::broker::run::run_cli(program.to_str().unwrap(), &[], Duration::from_secs(60))
    });
    assert_eq!(got.unwrap_err(), cancelscope::STOPPED);
    assert!(took < Duration::from_secs(5), "took {took:?}");
}

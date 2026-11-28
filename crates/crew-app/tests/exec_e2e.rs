//! `crew exec` end to end: the real binary spawns itself as the broker, over the mock provider
//! (`CREW_BROKER_MOCK_REPLY`, keyless), and the answer lands on stdout alone.
use std::process::Command;

fn crew(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_crew"))
        .args(args)
        .current_dir(dir)
        .env("CREW_BROKER_MOCK_REPLY", "hello from the mock")
        .env("CREW_PROJECT_DIR", dir)
        // Hermetic, as `crew-plugin/tests/common`: no login-shell probe, no real ~/.crew.
        .env("CREW_SHELL_ENV", "0")
        .env("HOME", dir)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap()
}

fn scratch(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("crew-exec-e2e-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn exec_prints_the_answer_to_stdout_and_exits_0() {
    let d = scratch("ok");
    let out = crew(&d, &["exec", "say hi"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "hello from the mock"
    );
}

#[test]
fn exec_with_no_prompt_is_a_usage_error() {
    let d = scratch("usage");
    let out = crew(&d, &["exec"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

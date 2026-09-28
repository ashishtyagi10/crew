use super::*;

/// The most likely failure for a CLI agent, and the one that used to read
/// "produced no output". crew adds these agents on PATH presence alone —
/// there is no cheap way to ask whether a session is valid — so installed
/// and usable are different things and the message has to bridge them.
#[test]
fn a_signed_out_cli_says_so_and_says_what_to_do() {
    for msg in [
        "Error: Not authenticated. Please run `claude login`.",
        "unauthorized (401)",
        "You must sign in first",
        "error: no API key found",
    ] {
        let e = explain_failure("claude", msg);
        assert!(e.contains("not signed in"), "{msg} -> {e}");
        assert!(e.contains("run `claude` once"), "{msg} -> {e}");
    }
}

/// Anything else keeps the honest old wording, with whatever the CLI
/// actually said appended — silence was never the useful part.
#[test]
fn other_failures_carry_the_last_line_of_stderr() {
    let e = explain_failure("codex", "boot\nsomething exploded");
    assert!(e.starts_with("codex: produced no output"), "{e}");
    assert!(e.contains("something exploded"), "{e}");
    // Truly silent stays exactly as it was.
    assert_eq!(
        explain_failure("codex", "   \n  "),
        "codex: produced no output"
    );
}

/// A runaway stderr must not become the whole error message.
#[test]
fn a_huge_stderr_is_trimmed() {
    let e = explain_failure("x", &"y".repeat(10_000));
    assert!(e.chars().count() < 220, "len {}", e.chars().count());
}

/// End to end through the real child process: a program that writes an
/// auth error to stderr and nothing to stdout must come back explained,
/// not as silence. This is the path that made the whole change worth it.
#[test]
fn a_real_child_that_only_writes_stderr_is_explained() {
    let args = vec![
        "-c".into(),
        "printf 'Please run claude login\\n' >&2; exit 1".into(),
    ];
    let e = run_cli("sh", &args, Duration::from_secs(5)).unwrap_err();
    assert!(e.contains("not signed in"), "{e}");
}

#[test]
fn on_path_finds_sh() {
    assert!(on_path("sh"));
}

#[test]
fn on_path_rejects_missing() {
    assert!(!on_path("definitely-not-a-real-binary-xyz"));
}

#[test]
fn run_cli_captures_stdout() {
    let args = vec!["-c".into(), "printf hello".into()];
    assert_eq!(
        run_cli("sh", &args, Duration::from_secs(5)).unwrap(),
        "hello"
    );
}

#[test]
fn run_cli_times_out_on_hang() {
    let args = vec!["-c".into(), "sleep 5".into()];
    let r = run_cli("sh", &args, Duration::from_millis(150));
    assert!(r.unwrap_err().contains("timed out"));
}

/// opencode reads its project from `$PWD`. A broker whose host left `PWD`
/// behind (the app does: it sets the broker's directory, not the variable)
/// sent opencode into the wrong project, where it refused to read the real
/// one and stopped after a single step.
#[test]
fn an_agent_is_told_the_directory_it_really_runs_in() {
    let cmd = agent_command("opencode");
    let pwd = cmd
        .get_envs()
        .find(|(k, _)| *k == "PWD")
        .and_then(|(_, v)| v);
    let here = std::env::current_dir().unwrap();
    assert_eq!(pwd, Some(here.as_os_str()), "PWD not set to the real cwd");
}

/// The same, through a real child that prints `PWD` without a shell in
/// between (a shell would repair it and hide the bug). Under `cargo test`
/// the process runs in the crate's directory while `PWD` still names the
/// directory cargo was called from, so without the fix these differ.
#[cfg(unix)]
#[test]
fn a_real_child_sees_the_real_directory_as_pwd() {
    let out = run_cli("printenv", &["PWD".into()], Duration::from_secs(5)).unwrap();
    let here = std::env::current_dir().unwrap();
    assert_eq!(out.trim(), here.to_str().unwrap(), "child PWD is stale");
}

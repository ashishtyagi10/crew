use super::run_parts;
use crate::app::CrewApp;
use crate::cmdcheck::Verdict;
use crate::route::{shape_for, Shape};

#[test]
fn a_resolved_binary_is_wrapped_and_everything_else_is_interactive() {
    assert_eq!(shape_for(&Verdict::Executable("ls".into())), Shape::Wrapped);
    assert_eq!(
        shape_for(&Verdict::Builtin("export".into())),
        Shape::Interactive
    );
    assert_eq!(shape_for(&Verdict::No), Shape::Interactive);
}

/// Poll pane 0's pty until its output holds `marker`, returning everything
/// read; panics after a generous deadline so a hang names the marker.
#[cfg(unix)]
fn wait_for(app: &mut CrewApp, marker: &str) -> String {
    use crate::pane::PaneContent;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let mut seen = String::new();
    loop {
        if let Some(PaneContent::Terminal(t)) = app.panes.get_mut(0).map(|p| &mut p.content) {
            t.pty.start_capture();
            t.pty.try_read();
            seen.push_str(&t.pty.take_capture());
        }
        if seen.contains(marker) {
            return seen;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "never saw {marker:?} in: {seen:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// A builtin line with no idle shell runs in a fresh interactive shell, and
/// what it set is still there at the prompt that follows: the second line,
/// typed into the same pane, reads the variable the first one exported.
// Drives the user's real shell through a PTY: Unix-only by construction.
#[cfg(unix)]
#[test]
fn a_builtin_runs_in_a_shell_and_its_state_outlives_it() {
    let mut app = CrewApp::default();
    // `$X` is expanded by the shell, so the marker cannot come from the echo
    // of the typed line itself — only from the command having run.
    app.submit_input("export CREW_RUNPANE=alive; echo mark-${CREW_RUNPANE}-one".into());
    assert_eq!(app.panes.len(), 1);
    wait_for(&mut app, "mark-alive-one");
    // The pane is focused and idle-or-soon: the next bare line types into it.
    app.submit_input("echo mark-${CREW_RUNPANE}-two".into());
    assert_eq!(app.panes.len(), 1, "no second pane: the first is the shell");
    wait_for(&mut app, "mark-alive-two");
}

/// Shell syntax whose first word is no binary — a `for` loop — runs too.
#[cfg(unix)]
#[test]
fn a_keyword_line_runs_in_a_shell() {
    let mut app = CrewApp::default();
    app.submit_input("for i in 1 2; do echo loop-$i-x; done".into());
    assert_eq!(app.panes.len(), 1);
    let seen = wait_for(&mut app, "loop-2-x");
    assert!(seen.contains("loop-1-x"), "got: {seen:?}");
}

#[test]
fn labels_first_word_and_persists_shell_bash_wrapped() {
    let (label, program, script) = run_parts("npm test --watch", "/bin/zsh", Some("/bin/bash"));
    assert_eq!(label, "npm");
    assert_eq!(program, "/bin/bash");
    assert_eq!(script, "set -m; npm test --watch; exec /bin/zsh");
}

#[test]
fn labels_first_word_without_bash_falls_back_unwrapped() {
    let (label, program, script) = run_parts("npm test --watch", "/bin/zsh", None);
    assert_eq!(label, "npm");
    assert_eq!(program, "/bin/zsh");
    assert_eq!(script, "npm test --watch; exec /bin/zsh");
}

#[test]
fn handles_single_token() {
    let (label, program, script) = run_parts("htop", "/bin/sh", Some("/bin/bash"));
    assert_eq!(label, "htop");
    assert_eq!(program, "/bin/bash");
    assert!(script.starts_with("set -m; htop; exec "));
}

#[test]
fn empty_command_defaults_label() {
    // not reachable via `/run` (guarded), but the helper stays total.
    assert_eq!(run_parts("", "/bin/sh", Some("/bin/bash")).0, "run");
}

#[test]
fn label_derives_from_command_not_wrapper_program() {
    // The pane LABEL must come from the user's command, never from the
    // bash wrapper program that actually gets spawned.
    let (label, program, _) = run_parts("cargo build --release", "/bin/zsh", Some("/bin/bash"));
    assert_eq!(label, "cargo");
    assert_ne!(label, program);
}

#[test]
fn every_shell_gets_bash_wrapped_when_bash_present() {
    // Unlike the old allowlist, this no longer depends on the user's
    // shell basename at all — zsh, fish, whatever — bash wraps all of
    // them when it's available.
    for shell in [
        "/bin/zsh",
        "/bin/bash",
        "/bin/sh",
        "/usr/bin/dash",
        "/bin/ksh",
        "/usr/local/bin/fish",
    ] {
        let (_label, program, script) = run_parts("git status", shell, Some("/bin/bash"));
        assert_eq!(program, "/bin/bash", "shell {shell}");
        assert!(
            script.starts_with("set -m; "),
            "shell {shell} got: {script}"
        );
        assert!(
            script.ends_with(&format!("exec {shell}")),
            "shell {shell} got: {script}"
        );
    }
}

// Spawns a real shell: Unix-only, like the other spawn tests here.
#[cfg(unix)]
#[test]
fn submit_without_a_shell_opens_one() {
    // Unresolvable text with no panes at all used to hint ("not a command")
    // and drop the line. A bare line is a shell command and the shell is the
    // judge: it opens an interactive shell pane, which shows `command not
    // found` at a live prompt — what a terminal does.
    let mut app = CrewApp::default();
    assert!(!app.submit_input("definitely-not-a-command-xyz".to_string()));
    assert_eq!(app.panes.len(), 1, "a shell pane opened for the line");
    assert!(matches!(
        app.panes[0].content,
        crate::pane::PaneContent::Terminal(_)
    ));
    assert_eq!(
        app.panes[0].label.as_deref(),
        Some("definitely-not-a-command-xyz"),
        "labelled by the first word, like a wrapped run"
    );
    assert!(
        app.active_status().is_none(),
        "no hint: the pane is the answer"
    );
}

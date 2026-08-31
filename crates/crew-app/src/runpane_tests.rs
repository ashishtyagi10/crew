use super::run_parts;

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

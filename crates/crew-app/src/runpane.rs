//! `/run <cmd>`: launch a command in its own tiled pane that stays open after
//! the command exits (it re-execs the shell), so builds, tests, and long jobs
//! sit alongside your shells instead of blocking one.
use crate::app::CrewApp;
use crate::spawn::default_shell;
use std::path::Path;

/// Build the `(program, script)` to spawn a persistent pane. The invariant:
/// `body` must run as its own foreground process group so
/// `PtyTerm::foreground_pid()` — the signal that decides busy-diverts vs.
/// typing into an idle shell — can tell "a command is running" from "a prompt
/// is waiting". A plain `sh -c "body; exec shell"` runs non-interactively:
/// `body` shares the shell's own pgid, so once it forks/backgrounds or the
/// shell itself looks idle mid-command, foreground_pid() can misread the pane
/// as idle while `body` is still running — bar text meant for the pane then
/// types into the running program instead of diverting.
///
/// `set -m` (job control) is the fix — but only bash implements it fully.
/// Empirically verified on this machine: zsh's non-interactive `set -m`
/// creates the process group but never calls `tcsetpgrp` to hand the tty
/// over, so the spawned command stops (state `T`) the moment it reads from
/// the tty, while foreground_pid() still reads the pane as idle — `set -m`
/// on zsh breaks the command *and* fails the busy-detection it was meant to
/// enable. bash's `set -m` does the full handoff (tcgetpgrp lands on the
/// command's pgid, command runs `S`+). fish has no POSIX `set -m` at all
/// (`set` there is its variable-assignment builtin).
///
/// So the wrapper shell is decoupled from the user's shell: when `bash` is
/// `Some`, every persistent pane runs *that* bash as the program, with
/// `set -m; {body}; exec {user_shell}` as its script — bash does the job
/// control, and the trailing `exec` still drops the user into their own
/// shell once `body` finishes. When `bash` is `None` (no `/bin/bash` on the
/// host), this falls back to the unwrapped, pre-job-control form: program =
/// `user_shell`, script `{body}; exec {user_shell}`, no `set -m` — busy
/// detection degrades gracefully rather than risk running a command under a
/// shell whose job control is unverified.
fn persistent_wrapper(body: &str, user_shell: &str, bash: Option<&str>) -> (String, String) {
    match bash {
        Some(bash) => (
            bash.to_string(),
            format!("set -m; {body}; exec {user_shell}"),
        ),
        None => (user_shell.to_string(), format!("{body}; exec {user_shell}")),
    }
}

/// Probes for a bash binary to use as the job-control wrapper. macOS and
/// Linux always ship `/bin/bash`; this is the only host check — everything
/// else in this module stays pure and injectable for tests.
fn bash_path() -> Option<&'static str> {
    Path::new("/bin/bash").exists().then_some("/bin/bash")
}

/// Build the `(label, program, script)` for `/run <cmd>`: the label is the
/// command's first word (never the wrapper program); program and script come
/// from [`persistent_wrapper`].
pub(crate) fn run_parts(
    cmd: &str,
    user_shell: &str,
    bash: Option<&str>,
) -> (String, String, String) {
    let label = cmd.split_whitespace().next().unwrap_or("run").to_string();
    let (program, script) = persistent_wrapper(cmd, user_shell, bash);
    (label, program, script)
}

/// The `/diff` review script: a short status summary, the diff stat, then the
/// full colored working-tree diff — colors forced since git sees a pipe.
/// Returns `(program, script)` from [`persistent_wrapper`].
pub(crate) fn diff_script(user_shell: &str, bash: Option<&str>) -> (String, String) {
    let git = "git -c color.ui=always";
    let body = format!("{git} status --short; {git} --no-pager diff --stat; {git} --no-pager diff");
    persistent_wrapper(&body, user_shell, bash)
}

impl CrewApp {
    /// `/diff`: review the working tree's changes (à la Codex's `/diff`) in a
    /// tiled pane that drops to a fresh prompt after the diff prints.
    pub(crate) fn diff_in_pane(&mut self) {
        let shell = default_shell();
        let (program, script) = diff_script(&shell, bash_path());
        self.spawn_labeled_terminal(&program, &["-c".to_string(), script], "diff".to_string());
    }

    /// Spawn a pane running `cmd` in the user's shell and focus it.
    pub(crate) fn run_in_pane(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        if cmd.is_empty() {
            self.set_status("usage: /run <command>");
            return;
        }
        let shell = default_shell();
        let (label, program, script) = run_parts(cmd, &shell, bash_path());
        self.spawn_labeled_terminal(&program, &["-c".to_string(), script], label);
    }
}

#[cfg(test)]
mod tests {
    use super::{diff_script, run_parts};

    #[test]
    fn diff_script_forces_color_and_persists_shell_bash_wrapped() {
        let (program, script) = diff_script("/bin/zsh", Some("/bin/bash"));
        assert_eq!(program, "/bin/bash");
        assert!(script.contains("color.ui=always"), "got: {script}");
        assert!(script.contains("status --short"), "got: {script}");
        assert!(script.contains("diff --stat"), "got: {script}");
        assert!(script.starts_with("set -m; "), "got: {script}");
        assert!(script.ends_with("exec /bin/zsh"), "got: {script}");
    }

    #[test]
    fn diff_script_falls_back_unwrapped_without_bash() {
        let (program, script) = diff_script("/bin/zsh", None);
        assert_eq!(program, "/bin/zsh");
        assert!(!script.starts_with("set -m"), "got: {script}");
        assert!(script.contains("color.ui=always"), "got: {script}");
        assert!(script.ends_with("exec /bin/zsh"), "got: {script}");
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
            let (program, script) = diff_script(shell, Some("/bin/bash"));
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
}

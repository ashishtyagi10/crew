//! `/run <cmd>`: launch a command in its own tiled pane that stays open after
//! the command exits (it re-execs the shell), so builds, tests, and long jobs
//! sit alongside your shells instead of blocking one.
use crate::app::CrewApp;
use crate::spawn::default_shell;

/// Wrap `body` so it runs as the shell's own foreground job, then re-exec
/// `shell` for a fresh prompt afterward. `PtyTerm::foreground_pid()` — the
/// signal that decides busy-diverts vs. typing into an idle shell — reads the
/// process group in the foreground on the pty, not just "some child exists".
/// A plain `sh -c "body; exec shell"` runs non-interactively: `body` shares
/// the shell's own pgid, so once it forks/backgrounds or the shell itself
/// looks idle mid-command, foreground_pid() can misread the pane as idle
/// while `body` is still running — and bar text meant for the pane types
/// into the running program instead. `set -m` turns job control on, which
/// gives `body` its own foreground process group like an interactive shell
/// would, so foreground_pid() can tell "a command is running" from "a prompt
/// is waiting". This is allowlisted to `sh`-family shells (`set -m` is not
/// portable POSIX-shell syntax everywhere) — an unlisted shell (e.g. fish,
/// whose `set` is its variable-assignment builtin) falls back to the
/// unwrapped form: graceful degradation, not a hard requirement.
fn persistent_script(body: &str, shell: &str) -> String {
    let basename = shell.rsplit('/').next().unwrap_or(shell);
    if matches!(basename, "zsh" | "bash" | "sh" | "dash" | "ksh") {
        format!("set -m; {body}; exec {shell}")
    } else {
        format!("{body}; exec {shell}")
    }
}

/// Build the `(label, shell-script)` for `/run <cmd>`: the label is the command's
/// first word; the script runs the command then re-execs `shell` so the pane
/// persists with a fresh prompt afterward.
pub(crate) fn run_parts(cmd: &str, shell: &str) -> (String, String) {
    let label = cmd.split_whitespace().next().unwrap_or("run").to_string();
    (label, persistent_script(cmd, shell))
}

/// The `/diff` review script: a short status summary, the diff stat, then the
/// full colored working-tree diff — colors forced since git sees a pipe.
pub(crate) fn diff_script(shell: &str) -> String {
    let git = "git -c color.ui=always";
    let body = format!("{git} status --short; {git} --no-pager diff --stat; {git} --no-pager diff");
    persistent_script(&body, shell)
}

impl CrewApp {
    /// `/diff`: review the working tree's changes (à la Codex's `/diff`) in a
    /// tiled pane that drops to a fresh prompt after the diff prints.
    pub(crate) fn diff_in_pane(&mut self) {
        let shell = default_shell();
        let script = diff_script(&shell);
        self.spawn_labeled_terminal(&shell, &["-c".to_string(), script], "diff".to_string());
    }

    /// Spawn a pane running `cmd` in the user's shell and focus it.
    pub(crate) fn run_in_pane(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        if cmd.is_empty() {
            self.set_status("usage: /run <command>");
            return;
        }
        let shell = default_shell();
        let (label, script) = run_parts(cmd, &shell);
        self.spawn_labeled_terminal(&shell, &["-c".to_string(), script], label);
    }
}

#[cfg(test)]
mod tests {
    use super::{diff_script, persistent_script, run_parts};

    #[test]
    fn diff_script_forces_color_and_persists_shell() {
        let s = diff_script("/bin/zsh");
        assert!(s.contains("color.ui=always"), "got: {s}");
        assert!(s.contains("status --short"), "got: {s}");
        assert!(s.contains("diff --stat"), "got: {s}");
        assert!(s.starts_with("set -m; "), "got: {s}");
        assert!(s.ends_with("exec /bin/zsh"), "got: {s}");
    }

    #[test]
    fn labels_first_word_and_persists_shell() {
        let (label, script) = run_parts("npm test --watch", "/bin/zsh");
        assert_eq!(label, "npm");
        assert_eq!(script, "set -m; npm test --watch; exec /bin/zsh");
    }

    #[test]
    fn handles_single_token() {
        let (label, script) = run_parts("htop", "/bin/sh");
        assert_eq!(label, "htop");
        assert!(script.starts_with("set -m; htop; exec "));
    }

    #[test]
    fn empty_command_defaults_label() {
        // not reachable via `/run` (guarded), but the helper stays total.
        assert_eq!(run_parts("", "/bin/sh").0, "run");
    }

    #[test]
    fn sh_family_shells_get_job_control_prefix() {
        for shell in [
            "/bin/zsh",
            "/bin/bash",
            "/bin/sh",
            "/usr/bin/dash",
            "/bin/ksh",
        ] {
            let s = persistent_script("cmd", shell);
            assert!(
                s.starts_with("set -m; cmd; exec "),
                "shell {shell} got: {s}"
            );
        }
    }

    #[test]
    fn fish_does_not_get_job_control_prefix() {
        // fish's `set` is its variable-assignment builtin, not job control —
        // `set -m` there would be a syntax error, so unlisted shells degrade
        // gracefully to the unwrapped script instead.
        let s = persistent_script("cmd", "/usr/local/bin/fish");
        assert_eq!(s, "cmd; exec /usr/local/bin/fish");
    }
}

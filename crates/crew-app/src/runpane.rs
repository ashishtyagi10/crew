//! `/run <cmd>`: launch a command in its own tiled pane that stays open after
//! the command exits (it re-execs the shell), so builds, tests, and long jobs
//! sit alongside your shells instead of blocking one.
use crate::app::CrewApp;
use crate::spawn::default_shell;

/// Build the `(label, shell-script)` for `/run <cmd>`: the label is the command's
/// first word; the script runs the command then re-execs `shell` so the pane
/// persists with a fresh prompt afterward.
pub(crate) fn run_parts(cmd: &str, shell: &str) -> (String, String) {
    let label = cmd.split_whitespace().next().unwrap_or("run").to_string();
    (label, format!("{cmd}; exec {shell}"))
}

/// The `/diff` review script: a short status summary, the diff stat, then the
/// full colored working-tree diff — colors forced since git sees a pipe.
pub(crate) fn diff_script(shell: &str) -> String {
    let git = "git -c color.ui=always";
    format!(
        "{git} status --short; {git} --no-pager diff --stat; {git} --no-pager diff; exec {shell}"
    )
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
    use super::{diff_script, run_parts};

    #[test]
    fn diff_script_forces_color_and_persists_shell() {
        let s = diff_script("/bin/zsh");
        assert!(s.contains("color.ui=always"), "got: {s}");
        assert!(s.contains("status --short"), "got: {s}");
        assert!(s.contains("diff --stat"), "got: {s}");
        assert!(s.ends_with("exec /bin/zsh"), "got: {s}");
    }

    #[test]
    fn labels_first_word_and_persists_shell() {
        let (label, script) = run_parts("npm test --watch", "/bin/zsh");
        assert_eq!(label, "npm");
        assert_eq!(script, "npm test --watch; exec /bin/zsh");
    }

    #[test]
    fn handles_single_token() {
        let (label, script) = run_parts("htop", "/bin/sh");
        assert_eq!(label, "htop");
        assert!(script.starts_with("htop; exec "));
    }

    #[test]
    fn empty_command_defaults_label() {
        // not reachable via `/run` (guarded), but the helper stays total.
        assert_eq!(run_parts("", "/bin/sh").0, "run");
    }
}

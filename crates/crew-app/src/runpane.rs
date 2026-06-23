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

impl CrewApp {
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
    use super::run_parts;

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

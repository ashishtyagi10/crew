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

impl CrewApp {
    /// `/diff`: review the working tree's changes in the **file viewer**,
    /// where the diff rung pairs each removal with its replacement and marks
    /// the words that actually changed — a review rather than a scrollback of
    /// `git`'s own colours. The read runs on a worker thread
    /// ([`crate::diffjob`]) and the pane opens when it lands.
    ///
    /// The repo reviewed is the **focused pane's** directory, so `/diff` in a
    /// pane working in another checkout reviews that checkout.
    pub(crate) fn diff_in_pane(&mut self) {
        if self.diff_job.busy() {
            self.set_status("already reading the working tree\u{2026}");
            return;
        }
        let dir = self
            .panes
            .get(self.focused)
            .and_then(|p| p.dir.clone())
            .or_else(|| std::env::current_dir().ok());
        let Some(dir) = dir else {
            self.set_status("diff: no working directory");
            return;
        };
        self.set_status(format!("reading {}\u{2026}", dir.display()));
        self.diff_job.start(dir);
    }

    /// Open the finished review, or say why there is none. Drained once a
    /// tick from `poll_panes`; a no-op on every tick with nothing in flight.
    pub(crate) fn drain_diff_job(&mut self) -> bool {
        match self.diff_job.take() {
            Some(Ok(path)) => {
                let before = self.panes.len();
                self.open_view(&path.to_string_lossy());
                let repo = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let repo = repo.rsplit('-').next().unwrap_or("diff").to_string();
                self.name_last_view(&format!("diff \u{b7} {repo}"));
                self.mark_last_view_ephemeral(before);
                true
            }
            Some(Err(e)) => {
                self.set_status(e);
                true
            }
            None => false,
        }
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
#[path = "runpane_tests.rs"]
mod tests;

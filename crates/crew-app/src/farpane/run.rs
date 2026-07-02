//! Off-thread runner for the Far command line. The command executes in the
//! active panel's directory on a worker thread (never on the winit thread —
//! blocking there would freeze every pane) and reports back over an mpsc
//! channel that `poll_cmd` drains each tick.
use std::path::Path;
use std::sync::mpsc::{self, Receiver};

use super::keys::FarAction;
use super::FarPane;

/// What a finished command reports back: its exit code (None = killed by a
/// signal) and the last non-empty output line for the status bar.
pub(crate) struct CmdDone {
    pub code: Option<i32>,
    pub tail: String,
}

/// Run `sh_cmd` via `shell -c` in `cwd` on a worker thread; the result arrives
/// on the returned channel. Dropping the receiver just discards the result.
pub(crate) fn start(shell: &str, sh_cmd: &str, cwd: &Path) -> Receiver<CmdDone> {
    let (tx, rx) = mpsc::channel();
    let shell = shell.to_string();
    let sh_cmd = sh_cmd.to_string();
    let cwd = cwd.to_path_buf();
    std::thread::spawn(move || {
        let done = match std::process::Command::new(&shell)
            .arg("-c")
            .arg(&sh_cmd)
            .current_dir(&cwd)
            .output()
        {
            Ok(out) => CmdDone {
                code: out.status.code(),
                tail: tail_line(&out.stderr)
                    .or_else(|| tail_line(&out.stdout))
                    .unwrap_or_default(),
            },
            Err(e) => CmdDone {
                code: None,
                tail: format!("failed to start: {e}"),
            },
        };
        let _ = tx.send(done);
    });
    rx
}

/// The last non-empty line of `bytes`, if any — the one-line summary a status
/// bar can show.
fn tail_line(bytes: &[u8]) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
}

/// Run the typed command line against the active panel — never a new pane.
/// `cd` navigates the panel in place; anything else executes in the panel's
/// directory on a worker thread (the listing reloads when it finishes).
pub(crate) fn run_cmdline(p: &mut FarPane) -> FarAction {
    let cwd = p.active_cwd();
    let cmd = std::mem::take(&mut p.cmdline);
    let cmd = cmd.trim().to_string();
    if cmd.is_empty() {
        return FarAction::Status("nothing to run".into());
    }
    if let Some(target) = cd_target(&cmd) {
        return change_dir(p, &cwd, target);
    }
    if let Some((running, _)) = &p.running {
        return FarAction::Status(format!("still running ‘{running}’ — wait for it"));
    }
    let rx = start(&crate::spawn::default_shell(), &cmd, &cwd);
    let status = format!("running ‘{cmd}’ in {}…", cwd.display());
    p.running = Some((cmd, rx));
    FarAction::Status(status)
}

/// `cd <path>` from the command line: point the active panel at the target
/// (relative to its current directory; `~`/`$VAR` expand) without touching
/// the other panel.
fn change_dir(p: &mut FarPane, cwd: &Path, target: &str) -> FarAction {
    let dest = crate::pathexpand::expand_path(cwd, target);
    if !dest.is_dir() {
        return FarAction::Status(format!("cd: not a directory: {}", dest.display()));
    }
    let panel = p.active_panel_mut();
    panel.cwd = dest.clone();
    panel.sel = 0;
    panel.reload();
    FarAction::Status(format!("cd {}", dest.display()))
}

/// The `cd` target when the command line is a `cd` invocation: `cd` → `~`,
/// `cd <path>` → the path. `None` for every other command.
pub(crate) fn cd_target(cmd: &str) -> Option<&str> {
    let rest = cmd.trim().strip_prefix("cd")?;
    if rest.is_empty() {
        return Some("~");
    }
    rest.starts_with(char::is_whitespace).then(|| rest.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn wait(rx: Receiver<CmdDone>) -> CmdDone {
        rx.recv_timeout(Duration::from_secs(10))
            .expect("command result")
    }

    #[test]
    fn reports_exit_code_and_output_tail() {
        let done = wait(start("/bin/sh", "echo one; echo two", Path::new("/tmp")));
        assert_eq!(done.code, Some(0));
        assert_eq!(done.tail, "two");
    }

    #[test]
    fn stderr_wins_the_tail_and_failures_report_nonzero() {
        let done = wait(start(
            "/bin/sh",
            "echo out; echo err >&2; exit 3",
            Path::new("/tmp"),
        ));
        assert_eq!(done.code, Some(3));
        assert_eq!(done.tail, "err");
    }

    #[test]
    fn runs_in_the_given_directory() {
        let done = wait(start("/bin/sh", "pwd", Path::new("/")));
        assert_eq!(done.tail, "/");
    }

    #[test]
    fn cd_parsing() {
        assert_eq!(cd_target("cd"), Some("~"));
        assert_eq!(cd_target("cd "), Some("~"));
        assert_eq!(cd_target("cd src/app"), Some("src/app"));
        assert_eq!(cd_target("  cd ~/x  "), Some("~/x"));
        assert_eq!(cd_target("cdx"), None);
        assert_eq!(cd_target("ls"), None);
    }
}

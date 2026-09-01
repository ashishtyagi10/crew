use std::io::Write;
use std::path::Path;

use crate::app::{CrewApp, FALLBACK_SIZE};
use crate::layout::Rect;
use crate::pane::{spawn_pane, Pane, PaneContent, TermPane};
use crew_term::PtyTerm;

/// A zero rect; `build_frame`'s relayout assigns the real pane rect next frame.
pub(crate) const PLACEHOLDER_RECT: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 0.0,
    h: 0.0,
};

/// The user's preferred shell from `$SHELL`, falling back to
/// [`fallback_shell`]. `$SHELL` is a Unix convention and is normally unset on
/// Windows, so there it is an opt-in override (a Git-Bash user can point it at
/// their own shell) rather than the usual answer.
pub(crate) fn default_shell() -> String {
    std::env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(fallback_shell)
}

/// The shell to use when the user has expressed no preference, and the
/// last-resort second try in [`crate::pane::spawn_pane`].
#[cfg(unix)]
pub(crate) fn fallback_shell() -> String {
    "/bin/sh".to_string()
}

/// Windows has no `/bin/sh`: without this every pane failed to open with
/// "couldn't open shell", which is what made the platform build-but-not-run.
/// `%COMSPEC%` is set on every Windows install and names `cmd.exe`; the
/// literal is only for a stripped environment where even that is missing.
#[cfg(windows)]
pub(crate) fn fallback_shell() -> String {
    std::env::var("COMSPEC")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "cmd.exe".to_string())
}

/// The shell a new terminal pane opens first. On Windows this is PowerShell —
/// what Windows Terminal opens and what the platform's users expect — with
/// [`fallback_shell`]'s `cmd.exe` catching the (essentially impossible) host
/// with no PowerShell. On Unix the user's `$SHELL` is already the right answer.
#[cfg(windows)]
pub(crate) fn preferred_shell() -> String {
    std::env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "powershell.exe".to_string())
}

#[cfg(unix)]
pub(crate) fn preferred_shell() -> String {
    default_shell()
}

/// Env vars handed to labeled (run/diff/edit) pane spawns: the login-shell
/// PATH commands were *detected* against. The `-c` wrapper shell is non-login,
/// so without this a Dock-launched Crew (launchd's minimal PATH) routes
/// `claude` to a pane that can't find it — and the fallback interactive shell
/// sources rc files under the same broken PATH, spraying "command not found".
pub(crate) fn hydrated_env() -> Vec<(String, String)> {
    vec![("PATH".to_string(), crate::cmdcheck::effective_path())]
}

impl CrewApp {
    /// The directory new terminals start in — Crew's tracked working directory,
    /// the same one shown in the input-bar legend and moved by `cd`. `None` only
    /// before it has been seeded (e.g. in tests), so the child inherits ours.
    pub(crate) fn spawn_cwd(&self) -> Option<&Path> {
        (!self.cwd.as_os_str().is_empty()).then_some(self.cwd.as_path())
    }

    /// Spawn a new terminal pane and focus it.
    pub fn spawn_new_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        let shell = preferred_shell();
        match spawn_pane(&shell, &fallback_shell(), grid, self.spawn_cwd()) {
            Ok(pane) => {
                self.panes.push(pane);
                self.focus_new_pane();
                self.apply_notify_patterns();
            }
            // Surface the failure in the UI — stderr is invisible in the GUI.
            Err(e) => self.set_status(format!("couldn't open shell: {e}")),
        }
    }

    /// Spawn a labeled terminal pane running `command args` and focus it.
    pub fn spawn_labeled_terminal(&mut self, command: &str, args: &[String], label: String) {
        let cwd = self.spawn_cwd().map(std::path::Path::to_path_buf);
        self.spawn_labeled_terminal_in(command, args, label, cwd);
    }

    /// As [`Self::spawn_labeled_terminal`], but starts the pane in `cwd` (used by
    /// the Far command line to run in the active panel's directory). `None` falls
    /// back to the process's inherited directory.
    pub fn spawn_labeled_terminal_in(
        &mut self,
        command: &str,
        args: &[String],
        label: String,
        cwd: Option<std::path::PathBuf>,
    ) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        let env = hydrated_env();
        let env: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        match PtyTerm::spawn_with_env(grid, command, args, cwd.as_deref(), &env) {
            Ok(pty) => {
                let input = pty.writer();
                // rect/grid are placeholders; build_frame's relayout sizes the pane
                // to the content area (right of the sidebar) on the next frame.
                let pane = Pane {
                    glide: crate::glide::Glide::default(),
                    content: PaneContent::Terminal(Box::new(TermPane {
                        pty,
                        input,
                        cmd: None,
                        cmd_since: None,
                        tail: Default::default(),
                        read_at: 0,
                        spans: Default::default(),
                        trail: Default::default(),
                        images: Default::default(),
                    })),
                    grid,
                    rect: PLACEHOLDER_RECT,
                    label: Some(label),
                    name: None,
                    dir: cwd,
                    activity: false,
                    bell: false,
                    hidden: false,
                    attention: None,
                    born_ms: crate::anim::now_ms(),
                };
                self.panes.push(pane);
                self.focus_new_pane();
                self.apply_notify_patterns();
                self.redraw();
            }
            // Surface the failure in the UI — stderr is invisible in the GUI.
            Err(e) => self.set_status(format!("couldn't run {command}: {e}")),
        }
    }

    /// Send `text + newline` to the pane labeled `label` (if Terminal).
    pub fn send_to_label(&mut self, label: &str, text: &str) {
        for pane in &mut self.panes {
            if pane.label.as_deref() == Some(label) {
                if let PaneContent::Terminal(t) = &mut pane.content {
                    if let Err(e) = t
                        .input
                        .write_all(text.as_bytes())
                        .and_then(|_| t.input.write_all(b"\n"))
                        .and_then(|_| t.input.flush())
                    {
                        eprintln!("send_to_label write error: {e}");
                    }
                }
                return;
            }
        }
    }
}

#[cfg(test)]
#[path = "spawn_tests.rs"]
mod tests;

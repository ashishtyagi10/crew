use std::io::Write;
use std::path::Path;

use anyhow::Context;
use crew_render::CellView;
use crew_term::{GridSize, PtyTerm, TermModel};

use crate::chat::ChatPane;
use crate::farpane::FarPane;
use crate::layout::Rect;
use crate::mdpane::MdPane;
use crate::session::to_cellviews;
use crate::settingspane::SettingsPane;
use crate::swarmpane::SwarmPane;

/// Raw terminal pane: owns its PTY and writer.
pub struct TermPane {
    pub pty: PtyTerm,
    pub input: Box<dyn Write + Send>,
    /// Name of the foreground command running in the pane (e.g. `claude`),
    /// shown alongside the directory in the title. `None` when the shell is idle.
    /// Refreshed ~1×/s by `poll_panes` via [`crate::procname::ProcNames`].
    pub cmd: Option<String>,
    /// When the current foreground command (`cmd`) started, used to gate the
    /// "command finished" notification on a minimum runtime. `None` when idle.
    pub cmd_since: Option<std::time::Instant>,
}

/// Discriminated union of pane kinds. A handful of instances exist at once
/// (one per tile), so the size spread between variants is not worth the
/// match-site churn of boxing every large pane type.
#[allow(clippy::large_enum_variant)]
pub enum PaneContent {
    Terminal(Box<TermPane>),
    Chat(ChatPane),
    Settings(SettingsPane),
    Far(FarPane),
    Swarm(SwarmPane),
    Markdown(MdPane),
}

/// A single pane: owns its content, grid size, and pixel rect.
pub struct Pane {
    pub content: PaneContent,
    pub grid: GridSize,
    pub rect: Rect,
    /// Optional label for routing host actions to this pane.
    pub label: Option<String>,
    /// User-set pane name (via `/name`), shown in the title bar when present.
    pub name: Option<String>,
    /// The pane's working directory, if known — its folder name is shown as the
    /// title (below a `/name` override). Seeded at spawn and kept live: a `cd`
    /// inside the pane (reported via OSC 7, see `poll_panes`) updates it.
    pub dir: Option<std::path::PathBuf>,
    /// Unseen output since this pane was last focused (drives the activity dot).
    pub activity: bool,
    /// The program rang the bell since this pane was last focused.
    pub bell: bool,
    /// User-minimized into the left-nav PANES list (the `[-]` border button):
    /// excluded from the grid (and the LRU strip) until focused again. Named
    /// `hidden` because "minimized" already means the LRU bottom strip.
    pub hidden: bool,
    /// A "needs you" marker (bell / watched pattern / command finished) raised
    /// while the pane wasn't focused — blinks on its nav row, then holds steady
    /// until the pane is focused again. See the `attention` module.
    pub attention: Option<crate::attention::Attention>,
}

impl Pane {
    /// Short label for the pane's title bar: the user-set name if any, else the
    /// folder the pane was opened in, else the program-set title, else the kind.
    pub fn title_text(&self) -> String {
        if let Some(name) = &self.name {
            return name.clone();
        }
        match &self.content {
            PaneContent::Terminal(t) => {
                // Directory folder name, plus the foreground command when one is
                // running (e.g. `crew · claude`). The folder name wins over an OSC
                // title; the running command is appended to it.
                let dir = self.dir.as_deref().and_then(dir_label);
                match (dir, t.cmd.as_deref()) {
                    (Some(dir), Some(cmd)) => format!("{dir} · {cmd}"),
                    (Some(dir), None) => dir,
                    (None, Some(cmd)) => cmd.to_string(),
                    (None, None) => {
                        let ti = t.pty.title();
                        if ti.is_empty() {
                            "shell".into()
                        } else {
                            ti
                        }
                    }
                }
            }
            PaneContent::Chat(c) => {
                if c.show_source {
                    "chat · source".into()
                } else {
                    "chat".into()
                }
            }
            PaneContent::Settings(_) => "settings".into(),
            PaneContent::Far(_) => "far".into(),
            PaneContent::Swarm(_) => "swarm".into(),
            PaneContent::Markdown(m) => {
                let file = m
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| m.path.to_string_lossy().into_owned());
                format!("{file} · md")
            }
        }
    }

    /// Render this pane to a flat list of `CellView`s. `focused` brightens the
    /// terminal cursor (dim in unfocused panes).
    pub fn cells(&self, focused: bool) -> Vec<CellView> {
        match &self.content {
            PaneContent::Terminal(t) => to_cellviews(&t.pty.cells(focused)),
            PaneContent::Chat(c) => c.cells(self.grid.cols, self.grid.rows),
            PaneContent::Settings(s) => s.cells(self.grid.cols, self.grid.rows),
            PaneContent::Far(f) => f.cells(self.grid.cols, self.grid.rows),
            PaneContent::Swarm(s) => s.cells(self.grid.cols, self.grid.rows),
            PaneContent::Markdown(m) => m.cells(self.grid.cols, self.grid.rows),
        }
    }
}

/// The folder name to display for a pane opened in `dir`: the last path
/// component (e.g. `~/code/crew` → `crew`), falling back to the whole path for
/// roots like `/`. `None` for an empty path.
fn dir_label(dir: &Path) -> Option<String> {
    if dir.as_os_str().is_empty() {
        return None;
    }
    Some(match dir.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => dir.to_string_lossy().into_owned(),
    })
}

/// Spawn a terminal pane running a **login** shell (so the user's full shell
/// config — `.zprofile`/`.zshrc`, plugins, PATH — loads, like Ghostty/Terminal).
/// Tries `shell_primary` first and falls back to `shell_fallback`. When `cwd` is
/// given the shell starts in that directory.
pub fn spawn_pane(
    shell_primary: &str,
    shell_fallback: &str,
    grid: GridSize,
    cwd: Option<&Path>,
) -> anyhow::Result<Pane> {
    let login = ["-l".to_string()];
    let pty = PtyTerm::spawn_in(grid, shell_primary, &login, cwd)
        .or_else(|_| PtyTerm::spawn_in(grid, shell_fallback, &login, cwd))
        .with_context(|| {
            format!("failed to spawn shell (tried {shell_primary}, {shell_fallback})")
        })?;
    let input = pty.writer();
    Ok(Pane {
        content: PaneContent::Terminal(Box::new(TermPane {
            pty,
            input,
            cmd: None,
            cmd_since: None,
        })),
        grid,
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        dir: cwd.map(Path::to_path_buf),
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
    })
}

#[cfg(test)]
#[path = "pane_tests.rs"]
mod tests;

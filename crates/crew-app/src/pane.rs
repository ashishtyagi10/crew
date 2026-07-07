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

/// Discriminated union of pane kinds.
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
    })
}

/// Assign one pane's pixel rect and resize its PTY (Terminal only) when the
/// derived grid changes. Reserves a one-cell border ring (fieldset card).
pub fn relayout_one(pane: &mut Pane, rect: Rect, cell_w: f32, cell_h: f32) {
    pane.rect = rect;
    let cols = ((rect.w / cell_w).floor() as u16).saturating_sub(2).max(1);
    let rows = ((rect.h / cell_h).floor() as u16).saturating_sub(2).max(1);
    if cols != pane.grid.cols || rows != pane.grid.rows {
        let new_grid = GridSize { cols, rows };
        if let PaneContent::Terminal(t) = &mut pane.content {
            t.pty.resize(new_grid);
        }
        pane.grid = new_grid;
    }
}

/// Assign pixel rects to panes (zipped in order). Thin wrapper over `relayout_one`.
pub fn relayout(panes: &mut [Pane], rects: &[Rect], cell_w: f32, cell_h: f32) {
    for (pane, &rect) in panes.iter_mut().zip(rects.iter()) {
        relayout_one(pane, rect, cell_w, cell_h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CrewConfig;
    use crate::settingspane::SettingsPane;

    #[test]
    fn dir_label_is_the_folder_name() {
        assert_eq!(
            dir_label(Path::new("/Users/atyagi/code/crew")),
            Some("crew".to_string())
        );
        assert_eq!(dir_label(Path::new("/")), Some("/".to_string()));
        assert_eq!(dir_label(Path::new("")), None);
    }

    #[test]
    fn terminal_title_is_the_open_directory_folder() {
        let base = std::env::temp_dir().join("crew_pane_title_dir");
        std::fs::create_dir_all(&base).unwrap();
        let grid = GridSize { cols: 40, rows: 10 };
        let mut pane = spawn_pane("sh", "sh", grid, Some(&base)).unwrap();
        // The open directory's folder name is the title…
        assert_eq!(pane.title_text(), "crew_pane_title_dir");
        // …but an explicit /name still wins.
        pane.name = Some("build".into());
        assert_eq!(pane.title_text(), "build");
    }

    #[test]
    fn terminal_title_appends_the_foreground_command() {
        let base = std::env::temp_dir().join("crew_pane_title_cmd");
        std::fs::create_dir_all(&base).unwrap();
        let grid = GridSize { cols: 40, rows: 10 };
        let mut pane = spawn_pane("sh", "sh", grid, Some(&base)).unwrap();
        // Idle shell → just the folder name.
        assert_eq!(pane.title_text(), "crew_pane_title_cmd");
        // A running command rides alongside the directory.
        if let PaneContent::Terminal(t) = &mut pane.content {
            t.cmd = Some("claude".into());
        }
        assert_eq!(pane.title_text(), "crew_pane_title_cmd · claude");
        // A /name override still wins outright.
        pane.name = Some("build".into());
        assert_eq!(pane.title_text(), "build");
    }

    #[test]
    fn title_text_prefers_user_name() {
        let mut p = Pane {
            content: PaneContent::Settings(SettingsPane::new(CrewConfig::default(), vec![])),
            grid: GridSize { cols: 80, rows: 24 },
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
        };
        assert_eq!(p.title_text(), "settings");
        p.name = Some("build".into());
        assert_eq!(p.title_text(), "build");
    }
}

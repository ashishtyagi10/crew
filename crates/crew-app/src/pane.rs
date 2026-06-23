use std::io::Write;
use std::path::Path;

use anyhow::Context;
use crew_render::CellView;
use crew_term::{GridSize, PtyTerm, TermModel};

use crate::chat::ChatPane;
use crate::farpane::FarPane;
use crate::layout::Rect;
use crate::session::to_cellviews;
use crate::settingspane::SettingsPane;

/// Raw terminal pane: owns its PTY and writer.
pub struct TermPane {
    pub pty: PtyTerm,
    pub input: Box<dyn Write + Send>,
}

/// Discriminated union of pane kinds.
pub enum PaneContent {
    Terminal(Box<TermPane>),
    Chat(ChatPane),
    Settings(SettingsPane),
    Far(FarPane),
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
    /// Unseen output since this pane was last focused (drives the activity dot).
    pub activity: bool,
    /// The program rang the bell since this pane was last focused.
    pub bell: bool,
}

impl Pane {
    /// Short label for the pane's title bar: the user-set name if any, else the
    /// program-set title, else the pane kind.
    pub fn title_text(&self) -> String {
        if let Some(name) = &self.name {
            return name.clone();
        }
        match &self.content {
            PaneContent::Terminal(t) => {
                let ti = t.pty.title();
                if ti.is_empty() {
                    "shell".into()
                } else {
                    ti
                }
            }
            PaneContent::Chat(_) => "chat".into(),
            PaneContent::Settings(_) => "settings".into(),
            PaneContent::Far(_) => "far".into(),
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
        }
    }
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
        content: PaneContent::Terminal(Box::new(TermPane { pty, input })),
        grid,
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        activity: false,
        bell: false,
    })
}

/// Assign pixel rects to panes and resize each PTY (Terminal only) when its grid changes.
pub fn relayout(panes: &mut [Pane], rects: &[Rect], cell_w: f32, cell_h: f32) {
    for (pane, &rect) in panes.iter_mut().zip(rects.iter()) {
        pane.rect = rect;
        let cols = ((rect.w / cell_w).floor() as u16).max(1);
        // Reserve the top row for the pane title bar; the content grid is one
        // row shorter (PaneView shifts content down and draws the bar on row 0).
        let rows = ((rect.h / cell_h).floor() as u16).saturating_sub(1).max(1);
        if cols != pane.grid.cols || rows != pane.grid.rows {
            let new_grid = GridSize { cols, rows };
            if let PaneContent::Terminal(t) = &mut pane.content {
                t.pty.resize(new_grid);
            }
            pane.grid = new_grid;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CrewConfig;
    use crate::settingspane::SettingsPane;

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
            activity: false,
            bell: false,
        };
        assert_eq!(p.title_text(), "settings");
        p.name = Some("build".into());
        assert_eq!(p.title_text(), "build");
    }
}

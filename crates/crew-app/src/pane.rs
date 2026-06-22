use std::io::Write;

use anyhow::Context;
use crew_render::{CellView, PaneScene};
use crew_term::{GridSize, PtyTerm, TermModel};

use crate::chat::ChatPane;
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
}

/// A single pane: owns its content, grid size, and pixel rect.
pub struct Pane {
    pub content: PaneContent,
    pub grid: GridSize,
    pub rect: Rect,
    /// Optional label for routing host actions to this pane.
    pub label: Option<String>,
}

impl Pane {
    /// Render this pane to a flat list of `CellView`s.
    pub fn cells(&self) -> Vec<CellView> {
        match &self.content {
            PaneContent::Terminal(t) => to_cellviews(&t.pty.cells()),
            PaneContent::Chat(c) => c.cells(self.grid.cols, self.grid.rows),
            PaneContent::Settings(s) => s.cells(self.grid.cols, self.grid.rows),
        }
    }
}

/// Spawn a terminal pane running a **login** shell (so the user's full shell
/// config — `.zprofile`/`.zshrc`, plugins, PATH — loads, like Ghostty/Terminal).
/// Tries `shell_primary` first and falls back to `shell_fallback`.
pub fn spawn_pane(
    shell_primary: &str,
    shell_fallback: &str,
    grid: GridSize,
) -> anyhow::Result<Pane> {
    let login = ["-l".to_string()];
    let pty = PtyTerm::spawn_args(grid, shell_primary, &login)
        .or_else(|_| PtyTerm::spawn_args(grid, shell_fallback, &login))
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
    })
}

/// Assign pixel rects to panes and resize each PTY (Terminal only) when its grid changes.
pub fn relayout(panes: &mut [Pane], rects: &[Rect], cell_w: f32, cell_h: f32) {
    for (pane, &rect) in panes.iter_mut().zip(rects.iter()) {
        pane.rect = rect;
        let cols = ((rect.w / cell_w).floor() as u16).max(1);
        let rows = ((rect.h / cell_h).floor() as u16).max(1);
        if cols != pane.grid.cols || rows != pane.grid.rows {
            let new_grid = GridSize { cols, rows };
            if let PaneContent::Terminal(t) = &mut pane.content {
                t.pty.resize(new_grid);
            }
            pane.grid = new_grid;
        }
    }
}

/// Accent green for the focused pane's badge; muted grey otherwise.
const BADGE_ON: (u8, u8, u8) = (0, 255, 160);
const BADGE_OFF: (u8, u8, u8) = (110, 110, 120);

/// Append a single-digit index badge to the pane's top-right corner so the
/// Cmd+1..9 / Ctrl+Tab navigation is discoverable. Only shown for panes 1-9.
fn add_badge(cells: &mut Vec<CellView>, n: usize, cols: u16, focused: bool) {
    if cols < 3 || !(1..=9).contains(&n) {
        return;
    }
    let c = char::from_digit(n as u32, 10).unwrap_or('?');
    cells.push(CellView {
        col: cols - 2,
        row: 0,
        c,
        fg: if focused { BADGE_ON } else { BADGE_OFF },
        bg: (0, 0, 0),
        bold: focused,
        italic: false,
    });
}

/// Build a `Vec<PaneScene>` from the current pane state (for `renderer.frame`).
/// Each pane gets a corner index badge when more than one pane is open.
pub fn build_scenes(panes: &[Pane], focused: Option<usize>) -> Vec<PaneScene> {
    let multi = panes.len() > 1;
    panes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut cells = p.cells();
            if multi {
                add_badge(&mut cells, i + 1, p.grid.cols, focused == Some(i));
            }
            PaneScene {
                cells,
                x: p.rect.x,
                y: p.rect.y,
                w: p.rect.w,
                h: p.rect.h,
                focused: focused == Some(i),
                bordered: true,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_added_for_valid_index() {
        let mut cells = Vec::new();
        add_badge(&mut cells, 3, 40, true);
        assert!(cells
            .iter()
            .any(|c| c.c == '3' && c.row == 0 && c.col == 38 && c.fg == BADGE_ON));
    }

    #[test]
    fn no_badge_out_of_range_or_too_narrow() {
        let mut cells = Vec::new();
        add_badge(&mut cells, 0, 40, false);
        add_badge(&mut cells, 12, 40, false);
        add_badge(&mut cells, 2, 2, false);
        assert!(cells.is_empty());
    }
}

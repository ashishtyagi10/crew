use std::io::Write;

use anyhow::Context;
use crew_render::{CellView, PaneScene};
use crew_term::{GridSize, PtyTerm, TermModel};

use crate::chat::ChatPane;
use crate::layout::Rect;
use crate::session::to_cellviews;

/// Raw terminal pane: owns its PTY and writer.
pub struct TermPane {
    pub pty: PtyTerm,
    pub input: Box<dyn Write + Send>,
}

/// Discriminated union of pane kinds.
pub enum PaneContent {
    Terminal(Box<TermPane>),
    Chat(ChatPane),
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
        }
    }
}

/// Spawn a terminal pane, trying `shell_primary` first and falling back to `shell_fallback`.
pub fn spawn_pane(
    shell_primary: &str,
    shell_fallback: &str,
    grid: GridSize,
) -> anyhow::Result<Pane> {
    let pty = PtyTerm::spawn(grid, shell_primary)
        .or_else(|_| PtyTerm::spawn(grid, shell_fallback))
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

/// Build a `Vec<PaneScene>` from the current pane state (for `renderer.frame`).
pub fn build_scenes(panes: &[Pane], focused: usize) -> Vec<PaneScene> {
    panes
        .iter()
        .enumerate()
        .map(|(i, p)| PaneScene {
            cells: p.cells(),
            x: p.rect.x,
            y: p.rect.y,
            w: p.rect.w,
            h: p.rect.h,
            focused: i == focused,
        })
        .collect()
}

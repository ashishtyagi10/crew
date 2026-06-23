//! Scene-building: converts PaneScene slice into quads + per-pane Buffers.
use glyphon::Buffer;

use crate::cellgrid::{CellView, DEFAULT_BG};
use crate::celltext::{build_pane_buffer, FontParams};
use crate::quads::Quad;
use crate::roundborder::Border;

/// `(Buffer, origin_x, origin_y, pane_w, pane_h)` for one rendered pane.
pub(crate) type PaneBuffer = (Buffer, f32, f32, f32, f32);

/// One pane to be rendered: its cell data, pixel rect, and focus state.
pub struct PaneScene {
    pub cells: Vec<CellView>,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub focused: bool,
    /// Whether to draw the rounded GPU border. Surfaces that draw their own
    /// cell-based border (e.g. the input bar's titled card) set this `false`.
    pub bordered: bool,
    /// Overlay popups (command palette, help) drawn on top of everything. Their
    /// backgrounds and text are rendered in a second pass *after* base panes, so
    /// nothing behind them can bleed through — they are fully opaque.
    pub overlay: bool,
}

/// Unfocused panes use a plain mid-grey that stays visible on the black
/// background; the focused pane brightens to near-white (no neon tint).
const BORDER_NORMAL: [f32; 4] = [110.0 / 255.0, 110.0 / 255.0, 120.0 / 255.0, 1.0];
const BORDER_FOCUSED: [f32; 4] = [210.0 / 255.0, 210.0 / 255.0, 220.0 / 255.0, 1.0];
const BORDER_RADIUS: f32 = 10.0;
const BORDER_THICKNESS: f32 = 2.0;

/// Build all quads (cell backgrounds) and one Buffer per pane, plus rounded borders.
/// Returns `(quads, pane_buffers, borders)`. Only panes whose `overlay` flag
/// equals `want_overlay` are built, so the caller can render base panes and
/// overlay popups as two separate passes.
pub(crate) fn build_scene(
    panes: &[PaneScene],
    cell_w: f32,
    cell_h: f32,
    font_system: &mut glyphon::FontSystem,
    params: &FontParams,
    want_overlay: bool,
) -> (Vec<Quad>, Vec<PaneBuffer>, Vec<Border>) {
    let mut quads: Vec<Quad> = Vec::new();
    let mut buffers: Vec<PaneBuffer> = Vec::new();
    let mut borders: Vec<Border> = Vec::new();

    for pane in panes {
        if pane.overlay != want_overlay {
            continue;
        }
        let cols = ((pane.w / cell_w).floor() as usize).max(1);
        let rows = ((pane.h / cell_h).floor() as usize).max(1);

        // Background quads for cells with non-default bg colour.
        for cell in &pane.cells {
            if cell.bg != DEFAULT_BG {
                quads.push(Quad {
                    x: pane.x + f32::from(cell.col) * cell_w,
                    y: pane.y + f32::from(cell.row) * cell_h,
                    w: cell_w,
                    h: cell_h,
                    color: [
                        cell.bg.0 as f32 / 255.0,
                        cell.bg.1 as f32 / 255.0,
                        cell.bg.2 as f32 / 255.0,
                        1.0,
                    ],
                });
            }
        }

        // Rounded-corner border for this pane (unless it draws its own).
        if pane.bordered {
            let color = if pane.focused {
                BORDER_FOCUSED
            } else {
                BORDER_NORMAL
            };
            borders.push(Border {
                x: pane.x,
                y: pane.y,
                w: pane.w,
                h: pane.h,
                radius: BORDER_RADIUS,
                thickness: BORDER_THICKNESS,
                color,
            });
        }

        // One text Buffer per pane.
        let buf = build_pane_buffer(font_system, &pane.cells, cols, rows, pane.w, pane.h, params);
        buffers.push((buf, pane.x, pane.y, pane.w, pane.h));
    }

    (quads, buffers, borders)
}

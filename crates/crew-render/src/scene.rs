//! Scene-building: converts PaneScene slice into quads + per-pane Buffers.
use glyphon::Buffer;

use crate::cellgrid::{CellView, DEFAULT_BG};
use crate::celltext::{build_pane_buffer, FontParams};
use crate::quads::Quad;

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
}

const BORDER_PX: f32 = 2.0;
const BORDER_NORMAL: [f32; 4] = [40.0 / 255.0, 80.0 / 255.0, 95.0 / 255.0, 1.0];
const BORDER_FOCUSED: [f32; 4] = [1.0, 0.0, 170.0 / 255.0, 1.0];

/// Build all quads (cell backgrounds + borders) and one Buffer per pane.
/// Returns `(quads, pane_buffers)` where each entry in `pane_buffers` is
/// `(Buffer, origin_x, origin_y, pane_w, pane_h)`.
pub(crate) fn build_scene(
    panes: &[PaneScene],
    cell_w: f32,
    cell_h: f32,
    font_system: &mut glyphon::FontSystem,
    params: &FontParams,
) -> (Vec<Quad>, Vec<PaneBuffer>) {
    let mut quads: Vec<Quad> = Vec::new();
    let mut buffers: Vec<PaneBuffer> = Vec::new();

    for pane in panes {
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

        // Border: 4 thin rects outlining the pane.
        let bc = if pane.focused {
            BORDER_FOCUSED
        } else {
            BORDER_NORMAL
        };
        push_border(&mut quads, pane.x, pane.y, pane.w, pane.h, bc);

        // One text Buffer per pane.
        let buf = build_pane_buffer(font_system, &pane.cells, cols, rows, pane.w, pane.h, params);
        buffers.push((buf, pane.x, pane.y, pane.w, pane.h));
    }

    (quads, buffers)
}

/// Push 4 border quads: top, bottom, left, right.
fn push_border(quads: &mut Vec<Quad>, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
    // Top
    quads.push(Quad {
        x,
        y,
        w,
        h: BORDER_PX,
        color,
    });
    // Bottom
    quads.push(Quad {
        x,
        y: y + h - BORDER_PX,
        w,
        h: BORDER_PX,
        color,
    });
    // Left
    quads.push(Quad {
        x,
        y,
        w: BORDER_PX,
        h,
        color,
    });
    // Right
    quads.push(Quad {
        x: x + w - BORDER_PX,
        y,
        w: BORDER_PX,
        h,
        color,
    });
}

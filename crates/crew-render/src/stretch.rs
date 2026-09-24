//! Frames that fill their rect exactly: nine-slice scaling for cell cards.
//!
//! A card's frame is drawn in cells, and a rect is rarely a whole number of
//! them, so the frame used to stop short of its rect by the remainder — up to
//! a cell on the right and a row at the bottom. That remainder was drawn as
//! gap: the space between two cards wandered with every window size and font
//! (15 px here, 32 there, in one window), where a designed layout keeps one
//! gutter everywhere.
//!
//! A stretched frame spends the remainder INSIDE itself instead, the way a
//! resizable image does: the last column and the last row move out to the
//! rect's far edges, and every rule that crossed the seam — the top and
//! bottom rules, the side rules — is continued across it by a quad cut from
//! the same pixel band the rule's glyph fills. The corners, the legends and
//! the buttons keep their cells; only the rules get longer.
use crate::cellgrid::CellView;
use crate::paint::Paint;
use crate::scene::PaneScene;

/// Where a stretched frame splits, and by how much it stretches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Split {
    /// The last column and row: everything at or past them moves out.
    pub lc: u16,
    pub lr: u16,
    /// Px inserted before the last column / row.
    pub sx: f32,
    pub sy: f32,
}

/// The split for `pane`, or `None` when it does not stretch — not a frame,
/// or nothing to spend (a frame whose cells already reach its far edges, or
/// overrun them mid-glide, draws exactly as it always did).
pub(crate) fn split(pane: &PaneScene, cell_w: f32, cell_h: f32) -> Option<Split> {
    if !pane.stretch || pane.cells.is_empty() {
        return None;
    }
    let lc = pane.cells.iter().map(|c| c.col).max()?;
    let lr = pane.cells.iter().map(|c| c.row).max()?;
    let sx = if lc > 0 {
        pane.w - f32::from(lc + 1) * cell_w
    } else {
        0.0
    };
    let sy = if lr > 0 {
        pane.h - f32::from(lr + 1) * cell_h
    } else {
        0.0
    };
    let (sx, sy) = (sx.max(0.0), sy.max(0.0));
    (sx > 0.0 || sy > 0.0).then_some(Split { lc, lr, sx, sy })
}

/// A column's (or row's) px offset from the frame's origin: past the split,
/// the stretch is added.
fn at(i: f32, last: u16, cell: f32, stretch: f32) -> f32 {
    i * cell
        + if i >= f32::from(last) - 1e-3 {
            stretch
        } else {
            0.0
        }
}

/// A paint rect's far edge: moved out only when it reaches INTO the last
/// column, so a rect ending at the split stays put.
fn at_end(i: f32, last: u16, cell: f32, stretch: f32) -> f32 {
    i * cell
        + if i > f32::from(last) + 1e-3 {
            stretch
        } else {
            0.0
        }
}

/// `pane` as up to four scenes — the body, the last column, the last row and
/// the far corner — each at its moved-out origin, with its cells re-indexed
/// to its own grid. The body carries all of the paint, remapped to where it
/// lands on the stretched frame.
pub(crate) fn parts(pane: &PaneScene, s: &Split, cell_w: f32, cell_h: f32) -> Vec<PaneScene> {
    let (lc, lr) = (s.lc, s.lr);
    let (bw, bh) = (f32::from(lc) * cell_w, f32::from(lr) * cell_h);
    let region = |right: bool, bottom: bool| {
        let cells: Vec<CellView> = pane
            .cells
            .iter()
            .filter(|c| (c.col >= lc && lc > 0) == right && (c.row >= lr && lr > 0) == bottom)
            .map(|c| CellView {
                col: if right { c.col - lc } else { c.col },
                row: if bottom { c.row - lr } else { c.row },
                ..c.clone()
            })
            .collect();
        PaneScene {
            cells,
            x: pane.x + if right { bw + s.sx } else { 0.0 },
            y: pane.y + if bottom { bh + s.sy } else { 0.0 },
            w: if right {
                pane.w - bw - s.sx
            } else if lc > 0 {
                bw
            } else {
                pane.w
            },
            h: if bottom {
                pane.h - bh - s.sy
            } else if lr > 0 {
                bh
            } else {
                pane.h
            },
            focused: pane.focused,
            overlay: pane.overlay,
            ..Default::default()
        }
    };
    let mut body = region(false, false);
    body.paint = pane
        .paint
        .iter()
        .map(|p| {
            let (x0, x1) = (
                at(p.x, lc, cell_w, s.sx),
                at_end(p.x + p.w, lc, cell_w, s.sx),
            );
            let (y0, y1) = (
                at(p.y, lr, cell_h, s.sy),
                at_end(p.y + p.h, lr, cell_h, s.sy),
            );
            Paint {
                x: x0 / cell_w,
                y: y0 / cell_h,
                w: (x1 - x0) / cell_w,
                h: (y1 - y0) / cell_h,
                ..*p
            }
        })
        .collect();
    let mut out = vec![body];
    for (right, bottom) in [(true, false), (false, true), (true, true)] {
        let part = region(right, bottom);
        if !part.cells.is_empty() {
            out.push(part);
        }
    }
    out
}

/// One continued rule: a px rect and the rule's colour.
pub(crate) type Bridge = (f32, f32, f32, f32, (u8, u8, u8));

/// The quads that carry every rule across the stretch: each row whose cell
/// before the split reaches right into a cell that reaches back left, and
/// each column whose cell above the split reaches down into one that reaches
/// back up. Cut from the band the rule's own glyph fills (`boxglyph::band`),
/// one pixel into the glyphs on either side so no seam shows.
pub(crate) fn bridges(pane: &PaneScene, s: &Split, cell_w: f32, cell_h: f32) -> Vec<Bridge> {
    let find = |col: u16, row: u16| pane.cells.iter().find(|c| c.col == col && c.row == row);
    let arms = |c: Option<&CellView>| c.and_then(|c| crate::boxglyph::arms(c.c).map(|a| (a, c.fg)));
    let (cw_px, ch_px) = (cell_w.round() as u32, cell_h.round() as u32);
    let mut out = Vec::new();
    if s.sx > 0.0 && s.lc > 0 {
        let x0 = (pane.x + f32::from(s.lc) * cell_w).floor() - 1.0;
        let x1 = (pane.x + f32::from(s.lc) * cell_w + s.sx).ceil() + 1.0;
        for row in 0..=s.lr {
            let (Some((l, fg)), Some((r, _))) = (arms(find(s.lc - 1, row)), arms(find(s.lc, row)))
            else {
                continue;
            };
            if l[1] == 0 || r[3] == 0 {
                continue;
            }
            // Where glyphon puts the cell's top: the text origin TRUNCATED
            // (cosmic-text's physical y), which for the first row of a part
            // — the top and bottom rules each are one — is the whole of it.
            let top = (pane.y + at(f32::from(row), s.lr, cell_h, s.sy)).trunc();
            let (lo, hi) = crate::boxglyph::band(ch_px, l[1], ch_px);
            out.push((x0, top + lo as f32, x1 - x0, (hi - lo) as f32, fg));
        }
    }
    if s.sy > 0.0 && s.lr > 0 {
        let y0 = (pane.y + f32::from(s.lr) * cell_h).floor() - 1.0;
        let y1 = (pane.y + f32::from(s.lr) * cell_h + s.sy).ceil() + 1.0;
        for col in 0..=s.lc {
            let (Some((u, fg)), Some((d, _))) = (arms(find(col, s.lr - 1)), arms(find(col, s.lr)))
            else {
                continue;
            };
            if u[2] == 0 || d[0] == 0 {
                continue;
            }
            // And its left: the origin floored into a whole pixel.
            let left = (pane.x + at(f32::from(col), s.lc, cell_w, s.sx)).floor();
            let (lo, hi) = crate::boxglyph::band(cw_px, u[2], ch_px);
            out.push((left + lo as f32, y0, (hi - lo) as f32, y1 - y0, fg));
        }
    }
    out
}

#[cfg(test)]
#[path = "stretch_tests.rs"]
mod tests;

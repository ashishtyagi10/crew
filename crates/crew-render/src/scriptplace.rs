//! Where each run of joining script is drawn: a phrase at a time, on its
//! row's baseline.
//!
//! The cells a word is given are the sum of its characters' widths, and its
//! ink is rarely that wide: `हिंदी` is four cells and under three of ink,
//! `आज` two cells and nearly three. Each word drawn at its own first cell
//! put all its slack after it and squeezed every word too wide, so a
//! sentence read `भारत   ने आज` with `आज` shrunk. So the words of a phrase —
//! runs on one row, each one blank after the last — are laid together: at
//! their natural size, from the phrase's first cell, with equal gaps that
//! end the phrase at its last cell, but never wider than [`GAP_MAX`] cells.
//! A word's ink stays within a cell or two of its cells, so the cursor, a
//! selection and a search hit still sit on it. Only a phrase too tight for
//! [`GAP_MIN`] between its words falls back to each word in its own cells,
//! shrunk if it must be.
use std::collections::HashMap;
use std::sync::Arc;

use glyphon::{Buffer, FontSystem};

use crate::celltext::FontParams;
use crate::scriptrun::{RunArea, RunSpec};
use crate::scriptshape::{shaped, width};

/// The gap between two words of a phrase, at least and at most, in cells.
/// A Latin word's gap in the grid is one cell.
const GAP_MIN: f32 = 0.5;
const GAP_MAX: f32 = 1.5;

/// `specs` shaped and placed over `main`, the snapped buffer of the same
/// cells.
pub(crate) fn place(
    font_system: &mut FontSystem,
    main: &Buffer,
    specs: &[RunSpec],
    params: &FontParams,
) -> Vec<RunArea> {
    if specs.is_empty() {
        return Vec::new();
    }
    // Each row's top, and its baseline below that top, as the snapped
    // buffer laid them out: Hindi beside English sits on English's line.
    let mut rows: Vec<Option<(f32, f32)>> = Vec::new();
    for run in main.layout_runs() {
        if rows.len() <= run.line_i {
            rows.resize(run.line_i + 1, None);
        }
        rows[run.line_i] = Some((run.line_top, run.line_y - run.line_top));
    }
    let phrases =
        specs.chunk_by(|a, b| a.row == b.row && a.spare > 0 && a.col + a.cols + 1 == b.col);
    let mut out = Vec::with_capacity(specs.len());
    for phrase in phrases {
        for (s, (buf, base, x)) in phrase.iter().zip(lay(font_system, phrase, params)) {
            let row = usize::from(s.row);
            let fallback = (row as f32 * params.line_height, base);
            let (top, at) = rows.get(row).copied().flatten().unwrap_or(fallback);
            out.push(RunArea {
                ink: width(&buf),
                buf,
                dx: x.round(),
                dy: (top + at - base).round(),
                row: s.row,
                col: s.col,
                cols: s.cols,
                spare: s.spare,
            });
        }
    }
    out
}

/// Each word of `phrase` shaped, with its baseline and the x it starts at.
fn lay(
    font_system: &mut FontSystem,
    phrase: &[RunSpec],
    p: &FontParams,
) -> Vec<(Arc<Buffer>, f32, f32)> {
    let cell = p.cell_w;
    let (first, last) = (&phrase[0], &phrase[phrase.len() - 1]);
    let x0 = f32::from(first.col) * cell;
    // Where the phrase's cells end, and how far its last word may spill.
    let end = f32::from(last.col + last.cols) * cell;
    let most = f32::from(last.col) * cell + last.room(cell);
    let words: Vec<(Arc<Buffer>, f32)> = phrase
        .iter()
        .map(|s| shaped(font_system, s, p, f32::INFINITY))
        .collect();
    let inked: f32 = words.iter().map(|(b, _)| width(b)).sum();
    let gaps = (phrase.len() - 1) as f32;
    if inked + gaps * GAP_MIN * cell > most - x0 {
        return phrase
            .iter()
            .map(|s| {
                let (buf, base) = shaped(font_system, s, p, s.room(cell));
                (buf, base, f32::from(s.col) * cell)
            })
            .collect();
    }
    let gap = match gaps > 0.0 {
        true => ((end - x0 - inked) / gaps).clamp(GAP_MIN * cell, GAP_MAX * cell),
        false => 0.0,
    };
    let mut x = x0;
    words
        .into_iter()
        .map(|(buf, base)| {
            let at = x;
            x += width(&buf) + gap;
            (buf, base, at)
        })
        .collect()
}

/// Where the ink of a pane's runs sits against their cells, row by row, so a
/// rule a run wears — a link's underline, a strike — runs under its words
/// rather than under the cells its words moved off: a Hindi link's underline
/// ran on past its last word by the cells the phrase had drawn in. Each
/// segment maps a span of cells to a span of ink: a word's cells to its ink,
/// the blank between two words of a phrase to the space between their ink,
/// and the blanks after a word along with it.
#[derive(Default)]
pub(crate) struct Warp(HashMap<u16, Vec<Seg>>);

/// `(cells, ink)`: from-to x of each, off the pane's origin.
type Seg = ((f32, f32), (f32, f32));

impl Warp {
    pub fn of(runs: &[RunArea], cell_w: f32) -> Self {
        let mut rows: HashMap<u16, Vec<Seg>> = HashMap::new();
        let x = |col: u16| f32::from(col) * cell_w;
        for (i, r) in runs.iter().enumerate() {
            let (end, ink_end) = (r.col + r.cols, r.dx + r.ink);
            let segs = rows.entry(r.row).or_default();
            segs.push(((x(r.col), x(end)), (r.dx, ink_end)));
            if r.spare == 0 {
                continue;
            }
            let after = match runs.get(i + 1) {
                Some(n) if n.row == r.row && n.col == end + 1 => n.dx,
                _ => ink_end + f32::from(r.spare) * cell_w,
            };
            segs.push(((x(end), x(end + r.spare)), (ink_end, after)));
        }
        Self(rows)
    }

    /// `x..x + w` on `row`, off the pane's origin, moved with the ink.
    pub fn span(&self, row: u16, x: f32, w: f32) -> (f32, f32) {
        let Some(segs) = self.0.get(&row) else {
            return (x, w);
        };
        let at = |x: f32| {
            let hit = segs.iter().find(|((a, b), _)| *a <= x && x <= *b);
            hit.map_or(x, |((a, b), (i, j))| {
                i + (x - a) / (b - a).max(f32::EPSILON) * (j - i)
            })
        };
        let (from, to) = (at(x), at(x + w));
        (from, (to - from).max(0.0))
    }
}

#[cfg(test)]
#[path = "scriptcost_tests.rs"]
mod cost_tests;
#[cfg(test)]
#[path = "scriptplace_tests.rs"]
mod tests;

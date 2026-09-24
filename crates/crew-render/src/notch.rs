//! Where a card's legends break its glass rim.
//!
//! A fieldset frame is broken wherever its legend sits: the `─` rule stops,
//! the words stand in the gap. The glass sheet's rim runs along that same
//! rule — the sheet is inset to the stroke's centre — so without being told
//! where the legend is, the rim drew a bright line straight through its
//! words ("line going through the texts"). A notch is the sheet's copy of the
//! gap: the stretch of the top (or bottom) edge where the rule is not drawn
//! because something else is written there.
use crate::cellgrid::CellView;

/// Spans each edge carries at most; the shader reads a fixed number. More
/// legends than this on one edge merge across their narrowest rule.
pub const SPANS: usize = 4;

/// Notched stretches of a card's top and bottom edges, in px from the card's
/// left edge; `[0, 0]` is an empty slot. `depth` is how far a legend's row
/// reaches in from the edge — the fill fades in over it, so the sheet's edge
/// never crosses the words either.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Notch {
    pub top: [[f32; 2]; SPANS],
    pub bottom: [[f32; 2]; SPANS],
    pub depth: f32,
}

/// A cell the frame's own rule is drawn in: the box-drawing block, whose
/// `─` and corners the stroke is made of.
fn is_rule(c: char) -> bool {
    ('\u{2500}'..='\u{257F}').contains(&c)
}

/// Column runs of `row` that hold something other than the rule — within
/// the frame's corners (`1..cols - 1`), and only runs with a visible glyph:
/// a stretch the rule merely has not reached yet (a frame still assembling)
/// is not a legend.
fn runs(cells: &[CellView], row: u16, cols: usize) -> Vec<(usize, usize)> {
    if cols < 3 {
        return Vec::new();
    }
    // What stands in each column: None = blank, Some(true) = rule,
    // Some(false) = a glyph.
    let mut line: Vec<Option<bool>> = vec![None; cols];
    for c in cells.iter().filter(|c| c.row == row) {
        let col = usize::from(c.col);
        if col < cols && c.c != ' ' {
            line[col] = Some(is_rule(c.c));
        }
    }
    let mut out = Vec::new();
    let mut col = 1;
    while col < cols - 1 {
        if line[col] == Some(true) {
            col += 1;
            continue;
        }
        let start = col;
        let mut ink = false;
        while col < cols - 1 && line[col] != Some(true) {
            ink |= line[col] == Some(false);
            col += 1;
        }
        if ink {
            out.push((start, col));
        }
    }
    out
}

/// Fold `runs` into at most [`SPANS`] by bridging the narrowest gaps.
fn fit(mut runs: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    while runs.len() > SPANS {
        let i = (1..runs.len())
            .min_by_key(|&i| runs[i].0 - runs[i - 1].1)
            .unwrap_or(1);
        runs[i - 1].1 = runs[i].1;
        runs.remove(i);
    }
    runs
}

/// The notch for a card of `cols` × `rows` cells whose sheet starts `inset`
/// px in from the scene's left edge (the stroke's centre) and whose edge sits
/// `inset_y` px down from the top of its first row.
pub fn notch(
    cells: &[CellView],
    cols: usize,
    rows: usize,
    cell_w: f32,
    cell_h: f32,
    inset: f32,
    inset_y: f32,
) -> Notch {
    let px = |runs: Vec<(usize, usize)>| {
        let mut slots = [[0.0; 2]; SPANS];
        for (slot, (a, b)) in slots.iter_mut().zip(fit(runs)) {
            *slot = [a as f32 * cell_w - inset, b as f32 * cell_w - inset];
        }
        slots
    };
    let top = px(runs(cells, 0, cols));
    let bottom = if rows >= 2 {
        px(runs(cells, (rows - 1) as u16, cols))
    } else {
        [[0.0; 2]; SPANS]
    };
    Notch {
        top,
        bottom,
        depth: (cell_h - inset_y).max(0.0),
    }
}

#[cfg(test)]
#[path = "notch_tests.rs"]
mod tests;

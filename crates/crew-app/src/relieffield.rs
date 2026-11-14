//! A code block on a RELIEF theme (neumorphism): a well, not a slab.
//!
//! A fenced block is a tinted rectangle of cells (`chatfield`): the page
//! nudged toward the ink. On a soft theme that is a flat grey plate lying
//! on a card moulded out of light, the one hard-edged thing in the pane
//! (theme loop, 2026-11-14). It is read back off the cells, as the boxes
//! are (`reliefwell`): rows that carry the code ground from one column,
//! two or more running. The tint goes and a sunk sheet takes its place.
//! One row alone is an inline span, a chip in a sentence, and stays one.
use crew_render::CellView;

use crate::reliefwell::Box4;

/// The fewest tinted cells a row of a block has: under this it is a chip.
const MIN_W: usize = 8;
/// How much of a row's span must be tinted: a language badge may sit on
/// the field with a ground of its own, two chips on one line may not pass.
const COVER: f32 = 0.8;

/// The tinted span of each row that has one: `(row, first col, last col)`.
fn spans(cells: &[CellView], tint: (u8, u8, u8)) -> Vec<(u16, u16, u16)> {
    let mut rows: std::collections::BTreeMap<u16, (u16, u16, usize)> = Default::default();
    for c in cells.iter().filter(|c| c.bg == tint) {
        let e = rows.entry(c.row).or_insert((c.col, c.col, 0));
        *e = (e.0.min(c.col), e.1.max(c.col), e.2 + 1);
    }
    rows.into_iter()
        .filter(|(_, (lo, hi, n))| *n >= MIN_W && *n as f32 >= COVER * f32::from(hi - lo + 1))
        .map(|(row, (lo, hi, _))| (row, lo, hi))
        .collect()
}

/// The code blocks in `cells`: runs of two or more rows tinted `tint` from
/// the same column.
pub(crate) fn fields(cells: &[CellView], tint: (u8, u8, u8)) -> Vec<Box4> {
    let mut out: Vec<Box4> = Vec::new();
    let mut open: Option<Box4> = None;
    for (row, lo, hi) in spans(cells, tint) {
        match open.as_mut() {
            Some(b) if b.r1 + 1 == row && b.c0 == lo => {
                b.r1 = row;
                b.c1 = b.c1.max(hi);
            }
            _ => {
                out.extend(open.take().filter(|b| b.r1 > b.r0));
                open = Some(Box4 {
                    c0: lo,
                    r0: row,
                    c1: hi,
                    r1: row,
                });
            }
        }
    }
    out.extend(open.filter(|b| b.r1 > b.r0));
    out
}

/// Take the tint off `fields`: their cells stand on `page`, which is the
/// well's own floor.
pub(crate) fn clear(
    cells: &mut [CellView],
    fields: &[Box4],
    tint: (u8, u8, u8),
    page: (u8, u8, u8),
) {
    let within = |c: &CellView| {
        fields
            .iter()
            .any(|f| (f.r0..=f.r1).contains(&c.row) && (f.c0..=f.c1).contains(&c.col))
    };
    for c in cells.iter_mut().filter(|c| c.bg == tint) {
        if within(c) {
            c.bg = page;
        }
    }
}

#[cfg(test)]
#[path = "relieffield_tests.rs"]
mod tests;

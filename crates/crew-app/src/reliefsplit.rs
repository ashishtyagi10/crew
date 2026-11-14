//! A box of several panels on a RELIEF theme: one well a panel.
//!
//! `/far` draws its two file panels as one box with a rule down the middle
//! (`┬` on the top edge over `┴` on the bottom). Pressed in whole
//! ([`crate::reliefwell`]) it was one wide well with the last ruled line in
//! the pane standing in it, dark on a pale card and white on slate (theme
//! loop, 2026-11-14). The rule goes and each panel is a well of its own,
//! with a rib of card between them where the rule stood.
use crew_render::CellView;

use crate::reliefwell::Box4;

const TEE_DOWN: char = '\u{252c}';
const TEE_UP: char = '\u{2534}';

/// The columns where `b` is divided: a `┬` on its top edge straight over a
/// `┴` on its bottom one.
pub(crate) fn joints(cells: &[CellView], b: &Box4) -> Vec<u16> {
    let on = |ch: char, row: u16| -> Vec<u16> {
        cells
            .iter()
            .filter(|c| c.c == ch && c.row == row && c.col > b.c0 && c.col < b.c1)
            .map(|c| c.col)
            .collect()
    };
    let bottom = on(TEE_UP, b.r1);
    let mut v: Vec<u16> = on(TEE_DOWN, b.r0)
        .into_iter()
        .filter(|c| bottom.contains(c))
        .collect();
    v.sort_unstable();
    v
}

/// `b` as its panels: the box cut at `joints`. A panel ends a column short
/// of the rule and the next one starts ON it: the rule's column was the
/// second panel's margin, and a well starting past it stood its rim on the
/// panel's first glyphs.
pub(crate) fn panels(b: &Box4, joints: &[u16]) -> Vec<Box4> {
    let mut out = Vec::new();
    let mut c0 = b.c0;
    for &j in joints {
        out.push(Box4 {
            c0,
            c1: j - 1,
            ..*b
        });
        c0 = j;
    }
    out.push(Box4 { c0, ..*b });
    out.retain(|p| p.c1 > p.c0);
    out
}

/// Blank the dividing rules of `b` at `joints`, top edge to bottom.
pub(crate) fn undivide(cells: &mut [CellView], b: &Box4, joints: &[u16]) {
    for c in cells.iter_mut() {
        let rule = ('\u{2500}'..='\u{257f}').contains(&c.c);
        if rule && joints.contains(&c.col) && (b.r0..=b.r1).contains(&c.row) {
            c.c = ' ';
        }
    }
}

#[cfg(test)]
#[path = "reliefsplit_tests.rs"]
mod tests;

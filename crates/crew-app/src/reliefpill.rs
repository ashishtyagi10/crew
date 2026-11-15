//! A highlighted row inside a well is a pill, not a slab.
//!
//! On the soft themes a box a pane draws is pressed into its card as a well
//! ([`crate::reliefwell`]), but the row a list highlights in it — `/far`'s
//! cursor bar, the settings form's focused row — was still a band of cell
//! backgrounds running square-ended from one side of the well to the
//! other: a flat slab laid in a dish (the open list of 2026-11-14). On a
//! moulded surface a choice is a pill lifted inside the dish, the way
//! neumorphic lists draw a selection. Crew already draws that shape for
//! inline code: a [`CellView::mark`] run is its own capsule, all four
//! corners round and inset from its row. So a band that fills a well's row
//! is handed that shape; its colour, its letters and every hit path stay
//! as they were.
use crew_render::CellView;

use crate::reliefwell::Box4;

/// Turn each full-width band of colour inside `wells` into a capsule.
/// A band is a row whose every cell between the well's sides has the same
/// background, other than `page`.
pub(crate) fn pill(cells: &mut [CellView], wells: &[Box4], page: (u8, u8, u8)) {
    for w in wells {
        if w.c1 < w.c0 + 2 {
            continue;
        }
        for row in w.r0 + 1..w.r1 {
            let band = |c: &CellView| c.row == row && c.col > w.c0 && c.col < w.c1;
            let mut bgs = cells.iter().filter(|c| band(c)).map(|c| c.bg);
            let Some(bg) = bgs.next() else {
                continue;
            };
            let count = 1 + bgs.clone().count();
            let whole = count == usize::from(w.c1 - w.c0 - 1);
            if bg != page && whole && bgs.all(|b| b == bg) {
                cells
                    .iter_mut()
                    .filter(|c| band(c))
                    .for_each(|c| c.mark = true);
            }
        }
    }
}

#[cfg(test)]
#[path = "reliefpill_tests.rs"]
mod tests;

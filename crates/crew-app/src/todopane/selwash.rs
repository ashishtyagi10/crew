//! The selected todo wears the selection wash, as every other list's
//! selected row does (the palette, the pickers, the inbox, `/far`).
//!
//! It was marked by a `›` and bold alone: on a tube, where every glyph
//! glows alike, the row the next key acts on was hard to find, and the
//! group heads (`#priya`) are bold too, so a head read as selected. An item
//! is a BLOCK — a wrapped title and, on a narrow pane, a stacked chip row —
//! so the wash covers every row of it, or the block reads as two items.
use crew_render::CellView;

use super::{measure::item_h, render::row_cells, TodoPane};

/// Draw item `idx` at `(row, cols, bottom)` and, when it is the selected
/// one, wash each of its rows above `bottom`. The last column is left on the
/// page: the scroll gutter rides it, and a thumb on the wash reads as part
/// of the row.
pub(super) fn item(
    out: &mut Vec<CellView>,
    p: &TodoPane,
    idx: usize,
    (row, cols, bottom): (u16, u16, u16),
    selected: bool,
    now_ms: u64,
) {
    row_cells(out, p, idx, (row, cols, bottom), selected, now_ms);
    if !selected {
        return;
    }
    let h = item_h(&p.items[idx], cols, now_ms, p.rowctx());
    for r in row..row.saturating_add(h).min(bottom) {
        crate::menuwash::wash_row(out, r, cols.saturating_sub(1));
    }
}

#[cfg(test)]
#[path = "selwash_tests.rs"]
mod tests;

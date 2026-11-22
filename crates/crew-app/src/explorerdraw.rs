//! Drawing the Explorer's tree: its rule, then one row per [`Item`] — indent guides down each
//! open folder, a chevron on a folder, the name. Row 0 of the panel is the rule; item `k` of the
//! scrolled list is on row `k + 1`, which is the mapping a click reads back
//! ([`crate::explorerhit`]).
use std::path::Path;

use crew_render::CellView;

use crate::explorer::Item;

/// Rows above the first item: the section's rule.
pub(crate) const TOP: u16 = 1;

/// The column an item at `depth` starts on: one in, two more per level.
fn x(depth: u16) -> u16 {
    1 + 2 * depth
}

/// The tree's cells for a panel `cols` × `rows`, the list scrolled by `scroll`. `hover` is the
/// item under the pointer, `picked` the file last opened from the tree.
pub(crate) fn cells(
    items: &[Item],
    scroll: usize,
    hover: Option<usize>,
    picked: Option<&Path>,
    (cols, rows): (u16, u16),
) -> Vec<CellView> {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let mut out =
        crate::boxdraw::section_header("EXPLORER", cols, t.border_normal, accent, t.page_bg);
    let shown = usize::from(rows.saturating_sub(TOP));
    for (k, item) in items.iter().enumerate().skip(scroll).take(shown) {
        let row = TOP + (k - scroll) as u16;
        // A guide under every open folder this item sits inside — the root's excepted, as VS
        // Code draws none for the workspace itself.
        for d in 1..item.depth {
            put(&mut out, x(d), row, cols, "\u{2502}", t.border_normal);
        }
        let lit = hover == Some(k);
        // Folders in the accent's readable ink, files in ink; the file last opened, and the row
        // under the pointer, in the accent itself.
        let fg = match () {
            _ if item.more.is_some() => t.text_muted,
            _ if lit || picked == Some(item.path.as_path()) => accent,
            _ if item.dir && item.depth > 0 => crate::accentink::accent_ink(),
            _ => t.ink,
        };
        let at = x(item.depth);
        if item.dir && item.more.is_none() {
            let chevron = if item.open { "\u{25be}" } else { "\u{25b8}" };
            put(&mut out, at, row, cols, chevron, t.text_muted);
        }
        // The root reads as VS Code's workspace heading: its name in capitals.
        let name = match item.depth {
            0 => item.name.to_uppercase(),
            _ => item.name.clone(),
        };
        put(&mut out, at + 2, row, cols, &name, fg);
    }
    out
}

/// Write `s` at `col` on `row`, clipped to the panel.
fn put(out: &mut Vec<CellView>, col: u16, row: u16, cols: u16, s: &str, fg: (u8, u8, u8)) {
    if col >= cols {
        return;
    }
    crate::navtext::put_at(out, s, col, row, cols, fg);
}

#[cfg(test)]
#[path = "explorerdraw_tests.rs"]
mod tests;

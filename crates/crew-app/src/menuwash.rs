//! The palette's and every picker's selected row wears the selection wash,
//! edge to edge, as the inbox's and `/far`'s do.
//!
//! It was marked by a `›` and bold alone: on a CRT tube, where every glyph
//! glows alike, the row was hard to find, and the section titles in a
//! picker (`anthropic`, `stored grants`) are bold too, so they read as
//! selected. The wash was left off once because it drowned the muted
//! description; the selected row's description is lifted to full ink now
//! (`cmdmenu`), and every colour on the wash is lifted to read on it. A
//! fuzzy-match chip and a colour swatch keep their own ground.
use crew_render::CellView;

/// Wash `row` of a `w`-column list: every cell on the page's ground takes
/// the wash and has its ink lifted to read on it; the columns nothing drew
/// in are filled, so the bar is whole.
pub(crate) fn wash_row(cells: &mut Vec<CellView>, row: u16, w: u16) {
    let t = crew_theme::theme();
    let wash = crew_theme::readable::selection_bg(&t);
    let floor = crew_theme::contrast::text_floor();
    let mut drawn = vec![false; usize::from(w)];
    for c in cells.iter_mut().filter(|c| c.row == row && c.col < w) {
        drawn[usize::from(c.col)] = true;
        if c.bg == t.page_bg && !c.mark {
            c.bg = wash;
            c.fg = crew_theme::readable::against(c.fg, wash, floor);
        }
    }
    for (col, _) in drawn.iter().enumerate().filter(|(_, d)| !**d) {
        cells.push(CellView {
            col: col as u16,
            row,
            c: ' ',
            fg: t.ink,
            bg: wash,
            ..Default::default()
        });
    }
}

#[cfg(test)]
#[path = "menuwash_tests.rs"]
mod tests;

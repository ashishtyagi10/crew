//! Search-match highlighting for `/find`: wash the background of cells whose
//! text matches the active search term in a pane's visible grid (smart-case),
//! so the match you scrolled to stands out — like Ghostty/WezTerm search.
use crew_render::CellView;

use crate::gridrows::grid_lines;

/// The ink a match's gutter tick wears: the highlight colour is a background
/// wash, and a single cell painted in it disappears against the page — so the
/// tick takes the wash's own hue at ink strength.
pub(crate) fn hit_mark() -> (u8, u8, u8) {
    let t = crew_theme::theme();
    crew_theme::readable::against(t.find_hl_bg, t.page_bg, 3.0)
}

/// Highlight every occurrence of `term` in the `cols × rows` grid `cells`,
/// smart-case (case-insensitive unless `term` has an uppercase letter). Returns
/// the number of matches highlighted. Builds the rows once, then washes once.
pub(crate) fn highlight(cells: &mut [CellView], term: &str, cols: u16, rows: u16) -> usize {
    let ci = !term.chars().any(char::is_uppercase);
    let fold = move |c: char| if ci { c.to_ascii_lowercase() } else { c };
    let needle: Vec<char> = term.chars().map(fold).collect();
    if needle.is_empty() || needle.len() > cols as usize {
        return 0;
    }
    // Collect matched (row, [start,end)) COLUMN ranges from one pass over the
    // rows. Matching runs over the row's characters, not its columns: the
    // grid is column-indexed, so `日本` sits there as `日 _ 本 _` and a needle
    // written `日本` never matched anything — `/find` could not find any text
    // holding a full-width character. `row_runs` drops the column a wide
    // glyph's second half owns and hands back where each kept character sits,
    // which is what the wash is applied by.
    let mut ranges: Vec<(u16, usize, usize)> = Vec::new();
    for (r, line) in grid_lines(cells, cols, rows).iter().enumerate() {
        let (chars, at) = crate::gridrows::row_runs(line);
        let folded: Vec<char> = chars.into_iter().map(fold).collect();
        let mut i = 0usize;
        while i + needle.len() <= folded.len() {
            if folded[i..i + needle.len()] == needle[..] {
                // One cell per character: the second column a full-width one
                // owns carries none, and the renderer widens the wash it
                // wears (`scene::cell_cols`).
                let last = at[i + needle.len() - 1] as usize;
                ranges.push((r as u16, at[i] as usize, last + 1));
                i += needle.len();
            } else {
                i += 1;
            }
        }
    }
    // The wash replaces the background the terminal already floored the
    // foreground against, so the ink has to be re-floored against the wash —
    // otherwise the one thing you searched for is the one thing you cannot
    // read. It showed on a TUI's painted row: the match came out as a solid
    // block with the text invisible inside it.
    let hl = crew_theme::theme().find_hl_bg;
    let floor = crew_theme::contrast::text_floor();
    for c in cells.iter_mut() {
        if ranges
            .iter()
            .any(|&(r, a, b)| c.row == r && (a..b).contains(&(c.col as usize)))
        {
            c.fg = crew_theme::readable::enforced(c.fg, hl, floor);
            c.bg = hl;
        }
    }
    ranges.len()
}

#[cfg(test)]
#[path = "findhl_tests.rs"]
mod tests;

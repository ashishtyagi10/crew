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
mod tests {
    use super::highlight;
    use crew_render::CellView;

    fn row(text: &str, r: u16) -> Vec<CellView> {
        text.chars()
            .enumerate()
            .map(|(i, c)| CellView {
                col: i as u16,
                row: r,
                c,
                fg: (200, 200, 200),
                bg: (0, 0, 0),
                bold: false,
                italic: false,
                ..Default::default()
            })
            .collect()
    }

    #[test]
    fn highlights_each_match_and_counts() {
        let _g = crate::app::theme_test_guard();
        // "foo bar foo" → two "foo" matches on row 0.
        let mut cells = row("foo bar foo", 0);
        let n = highlight(&mut cells, "foo", 11, 1);
        assert_eq!(n, 2);
        // exactly the 6 cells of the two matches are washed.
        let washed = cells
            .iter()
            .filter(|c| c.bg == crew_theme::theme().find_hl_bg)
            .count();
        assert_eq!(washed, 6);
        // a space between is not highlighted.
        assert!(cells
            .iter()
            .any(|c| c.c == ' ' && c.bg != crew_theme::theme().find_hl_bg));
    }

    #[test]
    fn smart_case_matches() {
        // lowercase term → case-insensitive.
        let mut cells = row("Error: boom", 0);
        assert_eq!(highlight(&mut cells, "error", 11, 1), 1);
        // a term with uppercase → case-sensitive (no match here).
        let mut cells = row("error: boom", 0);
        assert_eq!(highlight(&mut cells, "Error", 11, 1), 0);
    }

    /// `/find` could not find any text holding a full-width character: the
    /// grid is column-indexed, so `全角` sits on it as `全 _ 角 _` and a
    /// needle written `全角` never matched.
    #[test]
    fn a_needle_with_a_full_width_character_matches() {
        let _g = crate::app::theme_test_guard();
        // The grid as the terminal hands it over: one cell per character,
        // the column after a wide one left blank.
        let cells_of = |pairs: &[(u16, char)]| -> Vec<CellView> {
            pairs
                .iter()
                .map(|&(col, c)| CellView {
                    col,
                    row: 0,
                    c,
                    fg: (200, 200, 200),
                    bg: (0, 0, 0),
                    ..Default::default()
                })
                .collect()
        };
        // `ab全角cd` — 全 at 2-3, 角 at 4-5.
        let grid = [
            (0, 'a'),
            (1, 'b'),
            (2, '\u{5168}'),
            (4, '\u{89d2}'),
            (6, 'c'),
            (7, 'd'),
        ];
        let mut cells = cells_of(&grid);
        assert_eq!(highlight(&mut cells, "\u{5168}\u{89d2}", 8, 1), 1);
        let hl = crew_theme::theme().find_hl_bg;
        let washed: Vec<u16> = cells.iter().filter(|c| c.bg == hl).map(|c| c.col).collect();
        assert_eq!(washed, vec![2, 4], "both wide cells, and only those");

        // The neighbours are not swept in with them.
        let mut cells = cells_of(&grid);
        assert_eq!(highlight(&mut cells, "b\u{5168}", 8, 1), 1);
        let washed: Vec<u16> = cells.iter().filter(|c| c.bg == hl).map(|c| c.col).collect();
        assert_eq!(washed, vec![1, 2]);
    }

    /// The wash replaces the background the terminal floored the ink
    /// against. A match inside a TUI's painted row came out as a solid block
    /// with the text invisible in it — the one thing you searched for.
    #[test]
    fn a_match_stays_readable_over_the_wash() {
        let _g = crate::app::theme_test_guard();
        let hl = crew_theme::theme().find_hl_bg;
        // Ink the terminal picked to read on a LIGHT painted row, matched
        // inside it: over the wash it would be invisible.
        let mut cells = row("boom", 0);
        for c in cells.iter_mut() {
            c.fg = hl;
            c.bg = (230, 240, 255);
        }
        assert_eq!(highlight(&mut cells, "boom", 4, 1), 1);
        for c in &cells {
            let r = crew_theme::contrast_ratio(c.fg, c.bg);
            assert!(
                r >= crew_theme::contrast::text_floor() - 0.05,
                "{:?} reads at {r}",
                c.fg
            );
        }
    }

    #[test]
    fn empty_term_does_nothing() {
        let _g = crate::app::theme_test_guard();
        let mut cells = row("hello", 0);
        assert_eq!(highlight(&mut cells, "", 5, 1), 0);
        assert!(cells.iter().all(|c| c.bg != crew_theme::theme().find_hl_bg));
    }
}

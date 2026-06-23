//! Search-match highlighting for `/find`: wash the background of cells whose
//! text matches the active search term in a pane's visible grid (smart-case),
//! so the match you scrolled to stands out — like Ghostty/WezTerm search.
use crew_render::CellView;

use crate::gridrows::grid_lines;

/// Amber wash painted behind matched cells.
pub(crate) const HL_BG: (u8, u8, u8) = (90, 70, 0);

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
    // Collect matched (row, [start,end)) ranges from one pass over the rows.
    let mut ranges: Vec<(u16, usize, usize)> = Vec::new();
    for (r, line) in grid_lines(cells, cols, rows).iter().enumerate() {
        let folded: Vec<char> = line.iter().map(|&c| fold(c)).collect();
        let mut col = 0usize;
        while col + needle.len() <= cols as usize {
            if folded[col..col + needle.len()] == needle[..] {
                ranges.push((r as u16, col, col + needle.len()));
                col += needle.len();
            } else {
                col += 1;
            }
        }
    }
    for c in cells.iter_mut() {
        if ranges
            .iter()
            .any(|&(r, a, b)| c.row == r && (a..b).contains(&(c.col as usize)))
        {
            c.bg = HL_BG;
        }
    }
    ranges.len()
}

#[cfg(test)]
mod tests {
    use super::{highlight, HL_BG};
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
            })
            .collect()
    }

    #[test]
    fn highlights_each_match_and_counts() {
        // "foo bar foo" → two "foo" matches on row 0.
        let mut cells = row("foo bar foo", 0);
        let n = highlight(&mut cells, "foo", 11, 1);
        assert_eq!(n, 2);
        // exactly the 6 cells of the two matches are washed.
        let washed = cells.iter().filter(|c| c.bg == HL_BG).count();
        assert_eq!(washed, 6);
        // a space between is not highlighted.
        assert!(cells.iter().any(|c| c.c == ' ' && c.bg != HL_BG));
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

    #[test]
    fn empty_term_does_nothing() {
        let mut cells = row("hello", 0);
        assert_eq!(highlight(&mut cells, "", 5, 1), 0);
        assert!(cells.iter().all(|c| c.bg != HL_BG));
    }
}

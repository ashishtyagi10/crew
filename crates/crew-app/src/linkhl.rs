//! URL link colouring: tint http(s) URLs in terminal panes a distinct blue so
//! they read as clickable (Cmd+click opens them — see `openurl`). Applied to the
//! pane's visible cells each frame, like search highlighting.
use crew_render::CellView;

use crate::gridrows::grid_lines;
use crate::openurl::url_spans;

/// Foreground colour painted over URL cells.
pub(crate) const LINK_FG: (u8, u8, u8) = (90, 170, 255);

/// Recolour every cell that falls inside an http(s) URL on its row. Returns the
/// number of cells tinted. Builds the rows in one pass, then tints in one pass.
pub(crate) fn colorize(cells: &mut [CellView], cols: u16, rows: u16) -> usize {
    // (row, [start,end)) URL spans across the whole grid.
    let ranges: Vec<(u16, usize, usize)> = grid_lines(cells, cols, rows)
        .iter()
        .enumerate()
        .flat_map(|(r, line)| {
            url_spans(line)
                .into_iter()
                .map(move |(a, b)| (r as u16, a, b))
        })
        .collect();
    if ranges.is_empty() {
        return 0;
    }
    let mut tinted = 0;
    for c in cells.iter_mut() {
        if ranges
            .iter()
            .any(|&(r, a, b)| c.row == r && (a..b).contains(&(c.col as usize)))
        {
            c.fg = LINK_FG;
            tinted += 1;
        }
    }
    tinted
}

#[cfg(test)]
mod tests {
    use super::{colorize, LINK_FG};
    use crew_render::CellView;

    fn row(text: &str) -> Vec<CellView> {
        text.chars()
            .enumerate()
            .map(|(i, c)| CellView {
                col: i as u16,
                row: 0,
                c,
                fg: (200, 200, 200),
                bg: (0, 0, 0),
                bold: false,
                italic: false,
            })
            .collect()
    }

    #[test]
    fn tints_only_url_cells() {
        let line = "go https://ex.io/x done";
        let mut cells = row(line);
        let n = colorize(&mut cells, line.len() as u16, 1);
        let url = "https://ex.io/x";
        assert_eq!(n, url.len());
        // every URL cell is tinted...
        let start = line.find(url).unwrap();
        for c in &cells {
            let in_url = (start..start + url.len()).contains(&(c.col as usize));
            assert_eq!(c.fg == LINK_FG, in_url, "col {} mismatch", c.col);
        }
    }

    #[test]
    fn no_url_leaves_colors_untouched() {
        let mut cells = row("just plain text");
        assert_eq!(colorize(&mut cells, 15, 1), 0);
        assert!(cells.iter().all(|c| c.fg != LINK_FG));
    }
}

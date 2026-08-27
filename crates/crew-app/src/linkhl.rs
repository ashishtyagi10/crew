//! URL link colouring: tint http(s) URLs in terminal panes a distinct blue and
//! rule them so they read as clickable (Cmd+click opens them — see `openurl`).
//! Applied to the pane's visible cells each frame, like search highlighting.
use crew_render::CellView;
use crew_theme::deco::DecoLine;

use crate::gridrows::grid_lines;
use crate::openurl::url_spans;

/// Foreground colour painted over URL cells: the link blue, darkened until it
/// reads on the page it lands on. It shipped as the flat constant
/// `(90, 170, 255)`, which put a URL at 2.1 contrast on every light theme —
/// a link you could not read on a third of the themes.
pub(crate) fn link_fg() -> (u8, u8, u8) {
    crew_theme::readable::link(crew_theme::theme())
}

/// Recolour and underline every cell that falls inside an http(s) URL on its
/// row. Returns the number of cells tinted. Builds the rows in one pass, then
/// tints in one pass.
///
/// The underline is not decoration for its own sake: a link marked only by hue
/// is not marked at all for a reader who cannot separate that hue from the
/// body text, which is the same argument the gauges' shape cues make.
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
    let fg = link_fg();
    for c in cells.iter_mut() {
        if ranges
            .iter()
            .any(|&(r, a, b)| c.row == r && (a..b).contains(&(c.col as usize)))
        {
            c.fg = fg;
            c.deco.line = DecoLine::Single;
            tinted += 1;
        }
    }
    tinted
}

#[cfg(test)]
mod tests {
    use super::{colorize, link_fg, DecoLine};
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
                ..Default::default()
            })
            .collect()
    }

    #[test]
    fn tints_only_url_cells() {
        // The link colour is derived from the live theme; guard the global.
        let _g = crate::app::theme_test_guard();
        let line = "go https://ex.io/x done";
        let mut cells = row(line);
        let n = colorize(&mut cells, line.len() as u16, 1);
        let url = "https://ex.io/x";
        assert_eq!(n, url.len());
        // ...and ruled, so the link is legible without reading the hue.
        let ruled = cells.iter().filter(|c| c.deco.line == DecoLine::Single);
        assert_eq!(ruled.count(), url.len());
        // every URL cell is tinted...
        let start = line.find(url).unwrap();
        for c in &cells {
            let in_url = (start..start + url.len()).contains(&(c.col as usize));
            assert_eq!(c.fg == link_fg(), in_url, "col {} mismatch", c.col);
        }
    }

    #[test]
    fn no_url_leaves_colors_untouched() {
        // The link colour is derived from the live theme; guard the global.
        let _g = crate::app::theme_test_guard();
        let mut cells = row("just plain text");
        assert_eq!(colorize(&mut cells, 15, 1), 0);
        assert!(cells.iter().all(|c| c.fg != link_fg()));
    }
}

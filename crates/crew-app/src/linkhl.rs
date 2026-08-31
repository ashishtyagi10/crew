//! URL link colouring: tint http(s) URLs in terminal panes a distinct blue and
//! rule them so they read as clickable (Cmd+click opens them — see `openurl`).
//! Applied to the pane's visible cells each frame, like search highlighting.
use crew_render::CellView;
use crew_theme::deco::DecoLine;

use crate::openurl::url_spans;

/// Foreground colour painted over URL cells: the link blue, darkened until it
/// reads on the page it lands on. It shipped as the flat constant
/// `(90, 170, 255)`, which put a URL at 2.1 contrast on every light theme —
/// a link you could not read on a third of the themes.
pub(crate) fn link_fg() -> (u8, u8, u8) {
    crew_theme::readable::link(crew_theme::theme())
}

/// Recolour and underline every cell that falls inside an http(s) URL on its
/// row, given the pane's rows already read off the grid — the frame scans a
/// pane's cells ONCE and hands the same rows to every marker that wants them
/// (see `paneview`). Returns the number of cells tinted.
///
/// The underline is not decoration for its own sake: a link marked only by hue
/// is not marked at all for a reader who cannot separate that hue from the
/// body text, which is the same argument the gauges' shape cues make.
pub(crate) fn colorize_in(cells: &mut [CellView], lines: &[Vec<char>]) -> usize {
    // (row, [start,end)) URL spans across the whole grid.
    let ranges: Vec<(u16, usize, usize)> = lines
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

/// [`colorize_in`] against a pane's cells, scanning them here. The app scans
/// once per frame and calls the `_in` form; this is the seam the tests use.
#[cfg(test)]
pub(crate) fn colorize(cells: &mut [CellView], cols: u16, rows: u16) -> usize {
    colorize_in(cells, &crate::gridrows::grid_lines(cells, cols, rows))
}

#[cfg(test)]
#[path = "linkhl_tests.rs"]
mod tests;

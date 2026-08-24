//! Generic, pane-agnostic mouse selection over a rendered cell grid.
//!
//! Terminal panes select through alacritty's own grid model (see
//! [`crate::select`]). Every *other* pane kind — chat, settings, far, swarm —
//! renders to a flat `Vec<CellView>` with no selection of its own, so this
//! module provides a linear (reading-order) selection that works purely off the
//! rendered cells: it highlights the selected glyphs and extracts their text.
use std::collections::BTreeMap;

use crew_render::CellView;

/// An active selection over a non-terminal pane's rendered grid. Coordinates are
/// `(col, row)` cells; the selection is linear (row-major), like a terminal's.
#[derive(Clone, Copy)]
pub(crate) struct CellSel {
    pub pane: usize,
    pub anchor: (u16, u16),
    pub cursor: (u16, u16),
}

impl CellSel {
    /// `(start, end)` as inclusive `(row, col)` pairs in reading order, so a drag
    /// in either direction yields the same span.
    fn span(&self) -> ((u16, u16), (u16, u16)) {
        let a = (self.anchor.1, self.anchor.0);
        let b = (self.cursor.1, self.cursor.0);
        if a <= b {
            (a, b)
        } else {
            (b, a)
        }
    }

    /// Whether cell `(col, row)` falls within the linear selection.
    fn contains(&self, col: u16, row: u16) -> bool {
        let (s, e) = self.span();
        let p = (row, col);
        p >= s && p <= e
    }
}

/// Wash the background of every selected cell with `bg`. Only cells that carry a
/// glyph exist in `cells`, so this highlights the actual text, not empty space.
pub(crate) fn highlight(cells: &mut [CellView], sel: &CellSel, bg: (u8, u8, u8)) {
    for cell in cells.iter_mut() {
        if sel.contains(cell.col, cell.row) {
            cell.bg = bg;
        }
    }
}

/// Extract the selected text from `cells`: each selected row from its first to
/// its last selected glyph (gaps filled with spaces, trailing spaces trimmed),
/// joined with newlines. Empty when nothing is selected.
pub(crate) fn selection_text(cells: &[CellView], sel: &CellSel) -> String {
    let (s, e) = sel.span();
    let mut rows: BTreeMap<u16, BTreeMap<u16, char>> = BTreeMap::new();
    for cell in cells {
        if sel.contains(cell.col, cell.row) {
            rows.entry(cell.row).or_default().insert(cell.col, cell.c);
        }
    }
    let mut out: Vec<String> = Vec::new();
    for row in s.0..=e.0 {
        let line = match rows.get(&row) {
            Some(m) => {
                let first = *m.keys().next().unwrap();
                let last = *m.keys().next_back().unwrap();
                (first..=last)
                    .map(|c| *m.get(&c).unwrap_or(&' '))
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            }
            None => String::new(),
        };
        out.push(line);
    }
    out.join("\n")
}

/// Characters that end a word for the double-click gesture. Mirrors
/// alacritty's default separator set, which is what the terminal panes beside
/// these use — so a double-click means the same thing on a chat transcript as
/// it does on a shell, and a path or a flag still comes back whole.
const SEPARATORS: &str = " \t,│`|:\"'()[]{}<>";

fn is_separator(c: char) -> bool {
    SEPARATORS.contains(c)
}

/// The glyphs on `row`, keyed by column. Non-terminal panes render to a flat
/// cell list with no notion of a line, so a row has to be gathered.
fn row_glyphs(cells: &[CellView], row: u16) -> BTreeMap<u16, char> {
    cells
        .iter()
        .filter(|c| c.row == row)
        .map(|c| (c.col, c.c))
        .collect()
}

/// The `(first_col, last_col)` of the word under `(col, row)`, or `None` when
/// that cell is blank or a separator — a double-click on empty space selects
/// nothing rather than the nearest word, which is what a terminal does too.
pub(crate) fn word_span(cells: &[CellView], col: u16, row: u16) -> Option<(u16, u16)> {
    let g = row_glyphs(cells, row);
    let word = |c: u16| g.get(&c).copied().filter(|&ch| !is_separator(ch)).is_some();
    if !word(col) {
        return None;
    }
    let mut lo = col;
    while lo > 0 && word(lo - 1) {
        lo -= 1;
    }
    let mut hi = col;
    while word(hi + 1) {
        hi += 1;
    }
    Some((lo, hi))
}

/// The `(first_col, last_col)` spanned by every glyph on `row` — the
/// triple-click gesture. `None` for a row with nothing drawn on it.
pub(crate) fn line_span(cells: &[CellView], row: u16) -> Option<(u16, u16)> {
    let g = row_glyphs(cells, row);
    Some((*g.keys().next()?, *g.keys().next_back()?))
}

#[cfg(test)]
#[path = "gridsel_tests.rs"]
mod tests;

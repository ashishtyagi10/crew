//! Build a terminal grid's rows as char buffers in a single pass over the cells.
//! Per-row consumers (URL tinting, search highlighting) would otherwise rescan
//! every cell for each row — O(rows·cells); this is O(cells).
use crew_render::CellView;

/// `rows` char-buffers of width `cols`, each blank-filled then populated from the
/// cells on that row. Cells outside the `cols × rows` bounds are ignored.
pub(crate) fn grid_lines(cells: &[CellView], cols: u16, rows: u16) -> Vec<Vec<char>> {
    let mut lines = vec![vec![' '; cols as usize]; rows as usize];
    for c in cells {
        if (c.row as usize) < lines.len() && (c.col as usize) < cols as usize {
            lines[c.row as usize][c.col as usize] = c.c;
        }
    }
    lines
}

/// A grid row as the characters actually ON it, with the column each one
/// sits in: the blank column a full-width character's second half owns is
/// dropped rather than read as a space.
///
/// The grid is column-indexed, so `日本` is `['日', ' ', '本', ' ']` — and a
/// search for `日本` therefore matched nothing, ever. `/find` could not find
/// any text containing a full-width character. The column each kept character
/// sits in comes back with it, so a hit maps straight back onto the cells it
/// has to wash.
pub(crate) fn row_runs(line: &[char]) -> (Vec<char>, Vec<u16>) {
    let mut chars = Vec::with_capacity(line.len());
    let mut cols = Vec::with_capacity(line.len());
    let mut skip = false;
    for (i, &c) in line.iter().enumerate() {
        if std::mem::take(&mut skip) {
            continue;
        }
        skip = unicode_width::UnicodeWidthChar::width(c) == Some(2);
        chars.push(c);
        cols.push(i as u16);
    }
    (chars, cols)
}

#[cfg(test)]
mod tests {
    use super::grid_lines;
    use crew_render::CellView;

    fn cell(col: u16, row: u16, c: char) -> CellView {
        CellView {
            col,
            row,
            c,
            fg: (0, 0, 0),
            bg: (0, 0, 0),
            bold: false,
            italic: false,
            ..Default::default()
        }
    }

    #[test]
    fn buckets_cells_into_rows_blank_filling_gaps() {
        let cells = [cell(0, 0, 'h'), cell(2, 0, 'i'), cell(1, 1, 'x')];
        let lines = grid_lines(&cells, 3, 2);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], vec!['h', ' ', 'i']);
        assert_eq!(lines[1], vec![' ', 'x', ' ']);
    }

    #[test]
    fn row_runs_drops_the_column_a_wide_glyph_owns() {
        let line: Vec<char> = "\u{65e5}\u{672c} x"
            .chars()
            .flat_map(|c| {
                if unicode_width::UnicodeWidthChar::width(c) == Some(2) {
                    vec![c, ' ']
                } else {
                    vec![c]
                }
            })
            .collect();
        // The grid: 日 _ 本 _ ␠ x  — six columns, four characters.
        assert_eq!(line.len(), 6);
        let (chars, cols) = super::row_runs(&line);
        assert_eq!(chars, vec!['\u{65e5}', '\u{672c}', ' ', 'x']);
        assert_eq!(cols, vec![0, 2, 4, 5]);
    }

    #[test]
    fn ignores_out_of_bounds_cells() {
        // cells past `cols`/`rows` are dropped, not panicking.
        let cells = [cell(9, 0, 'a'), cell(0, 9, 'b')];
        let lines = grid_lines(&cells, 3, 2);
        assert!(lines.iter().all(|l| l.iter().all(|&c| c == ' ')));
    }
}

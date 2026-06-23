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
    fn ignores_out_of_bounds_cells() {
        // cells past `cols`/`rows` are dropped, not panicking.
        let cells = [cell(9, 0, 'a'), cell(0, 9, 'b')];
        let lines = grid_lines(&cells, 3, 2);
        assert!(lines.iter().all(|l| l.iter().all(|&c| c == ' ')));
    }
}

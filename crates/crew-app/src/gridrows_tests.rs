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

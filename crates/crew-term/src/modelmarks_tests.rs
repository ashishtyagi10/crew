use super::{GridSize, HeadlessTerm, TermModel};

/// A combining mark is no cell of its own: alacritty keeps it on the cell
/// before it (`Cell::zerowidth`), and so must the cells crew draws, or `क्ष`
/// is drawn `कष` and `हिंदी` loses its nasal dot.
#[test]
fn a_cells_marks_come_through_and_take_no_column() {
    let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 2 });
    t.feed("क्ष हिंदी".as_bytes());
    let mut row: Vec<_> = t
        .cells(false)
        .into_iter()
        .filter(|c| c.row == 0 && c.c != ' ')
        .collect();
    row.sort_by_key(|c| c.col);
    let got: Vec<(u16, char, String)> = row
        .iter()
        .map(|c| (c.col, c.c, c.marks.chars().collect()))
        .collect();
    let want = [
        (0, 'क', "\u{94d}"),
        (1, 'ष', ""),
        (3, 'ह', ""),
        (4, '\u{93f}', "\u{902}"),
        (5, 'द', ""),
        (6, '\u{940}', ""),
    ];
    let want: Vec<(u16, char, String)> = want.iter().map(|&(c, ch, m)| (c, ch, m.into())).collect();
    assert_eq!(got, want);
}

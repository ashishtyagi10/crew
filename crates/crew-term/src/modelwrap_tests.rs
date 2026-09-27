use crate::{GridSize, HeadlessTerm, TermModel};

#[test]
fn a_line_longer_than_the_pane_marks_the_row_it_wraps_from() {
    let mut t = HeadlessTerm::new(GridSize { cols: 10, rows: 4 });
    t.feed(b"short\r\nabcdefghijklmno\r\nend");
    assert_eq!(t.wrapped_rows(), vec![false, true, false, false]);
}

#[test]
fn a_hard_newline_is_not_a_wrap() {
    let mut t = HeadlessTerm::new(GridSize { cols: 10, rows: 3 });
    t.feed(b"0123456789\r\nnext");
    assert_eq!(t.wrapped_rows()[0], false, "exactly full, then a newline");
}

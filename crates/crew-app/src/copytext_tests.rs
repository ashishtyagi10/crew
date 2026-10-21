use super::*;

#[test]
fn a_row_copies_with_its_marks_and_without_the_wide_tail() {
    let virama = Marks::of(['\u{94d}']);
    let cells = [
        (0, 0, 'क', virama),
        (0, 1, 'ष', Marks::default()),
        (0, 3, '日', Marks::default()),
        (0, 5, '本', Marks::default()),
        (0, 8, 'x', Marks::default()),
        (2, 1, 'y', Marks::default()),
    ];
    let rows = rows(cells);
    assert_eq!(line(&rows[&0], 0, 20), "क्ष 日本 x");
    assert_eq!(line(&rows[&2], 0, 4), " y");
    assert_eq!(line(&rows[&0], 1, 3), "ष 日");
}

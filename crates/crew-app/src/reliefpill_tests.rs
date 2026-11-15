use super::*;

const PAGE: (u8, u8, u8) = (230, 230, 232);
const BAR: (u8, u8, u8) = (200, 200, 204);

fn cell(row: u16, col: u16, bg: (u8, u8, u8)) -> CellView {
    CellView {
        row,
        col,
        c: 'x',
        fg: (0, 0, 0),
        bg,
        ..CellView::default()
    }
}

/// A well three rows tall over columns 0..=5: the inner row 1 is cols 1..=4.
fn well() -> Box4 {
    Box4 {
        c0: 0,
        r0: 0,
        c1: 5,
        r1: 2,
    }
}

#[test]
fn a_band_across_the_well_is_a_pill() {
    let mut cells: Vec<_> = (1..5).map(|c| cell(1, c, BAR)).collect();
    pill(&mut cells, &[well()], PAGE);
    assert!(cells.iter().all(|c| c.mark));
}

/// A chip or a word's highlight is not a row: it keeps its own shape.
#[test]
fn a_part_of_a_row_is_left_alone() {
    let mut cells: Vec<_> = (1..5)
        .map(|c| cell(1, c, if c < 3 { BAR } else { PAGE }))
        .collect();
    pill(&mut cells, &[well()], PAGE);
    assert!(cells.iter().all(|c| !c.mark));
}

#[test]
fn page_and_rows_outside_the_well_are_left_alone() {
    let mut cells: Vec<_> = (1..5).map(|c| cell(1, c, PAGE)).collect();
    cells.extend((1..5).map(|c| cell(3, c, BAR)));
    pill(&mut cells, &[well()], PAGE);
    assert!(cells.iter().all(|c| !c.mark));
}

/// A run of one colour that stops short of the far wall (cells past it not
/// drawn at all) is a chip on the row, not the row.
#[test]
fn a_band_short_of_the_wall_is_left_alone() {
    let mut cells: Vec<_> = (1..3).map(|c| cell(1, c, BAR)).collect();
    pill(&mut cells, &[well()], PAGE);
    assert!(cells.iter().all(|c| !c.mark));
}

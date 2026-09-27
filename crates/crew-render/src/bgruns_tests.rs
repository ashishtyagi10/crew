use super::*;

const PAGE: (u8, u8, u8) = (250, 250, 250);
const A: (u8, u8, u8) = (0, 120, 80);
const B: (u8, u8, u8) = (200, 40, 40);

fn cell(col: u16, row: u16, c: char, bg: (u8, u8, u8)) -> CellView {
    CellView {
        col,
        row,
        c,
        bg,
        ..Default::default()
    }
}

fn row_of(row: u16, from: u16, to: u16, bg: (u8, u8, u8)) -> Vec<CellView> {
    (from..to).map(|c| cell(c, row, 'x', bg)).collect()
}

/// A highlight on bare page is one run, a capsule: all four corners round.
#[test]
fn a_lone_highlight_is_one_capsule() {
    let got = runs(&row_of(1, 2, 7, A), 10, 3, PAGE);
    assert_eq!(
        got,
        vec![Run {
            row: 1,
            col: 2,
            cols: 5,
            bg: A,
            round: [true; 4],
            mark: false,
        }]
    );
}

/// Two colours that abut keep a square seam between them; only the outer
/// ends round (a powerline prompt, a heatmap's squares).
#[test]
fn abutting_colours_keep_their_seam() {
    let mut cells = row_of(0, 0, 3, A);
    cells.extend(row_of(0, 3, 6, B));
    let got = runs(&cells, 8, 1, PAGE);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].round, [true, false, false, true], "A: left end only");
    assert_eq!(
        got[1].round,
        [false, true, true, false],
        "B: right end only"
    );
}

/// A block several rows tall rounds its four OUTER corners and nothing in
/// between — no scalloped edge down its sides.
#[test]
fn a_tall_block_rounds_only_its_outer_corners() {
    let cells: Vec<_> = (0..3).flat_map(|r| row_of(r, 1, 5, A)).collect();
    let got = runs(&cells, 6, 3, PAGE);
    let corners: Vec<_> = got.iter().map(|r| r.round).collect();
    assert_eq!(
        corners,
        vec![
            [true, true, false, false],
            [false, false, false, false],
            [false, false, true, true],
        ]
    );
}

/// Page-coloured cells draw nothing and break a run; a wide character covers
/// both of its columns.
#[test]
fn page_cells_break_runs_and_wide_chars_span_two() {
    let cells = vec![
        cell(0, 0, 'x', A),
        cell(1, 0, 'x', PAGE),
        cell(2, 0, '漢', A),
    ];
    let got = runs(&cells, 6, 1, PAGE);
    assert_eq!(
        got.iter().map(|r| (r.col, r.cols)).collect::<Vec<_>>(),
        [(0, 1), (2, 2)]
    );
}

/// A cell past the grid it was given is dropped rather than indexing out.
#[test]
fn cells_off_the_grid_are_ignored() {
    assert!(runs(&[cell(9, 0, 'x', A), cell(0, 5, 'x', A)], 4, 2, PAGE).is_empty());
}

/// Two marks stacked in the same columns — `car` matched on two rows — stay
/// two capsules; the same colour as a plain band still merges into one block.
#[test]
fn stacked_marks_stay_apart_and_stacked_bands_still_merge() {
    let mark = |row| -> Vec<CellView> {
        row_of(row, 2, 5, A)
            .into_iter()
            .map(|c| CellView { mark: true, ..c })
            .collect()
    };
    let got = runs(&[mark(1), mark(2)].concat(), 10, 4, PAGE);
    assert_eq!(got.len(), 2);
    assert!(
        got.iter().all(|r| r.mark && r.round == [true; 4]),
        "{got:?}"
    );
    let band = runs(
        &[row_of(1, 2, 5, A), row_of(2, 2, 5, A)].concat(),
        10,
        4,
        PAGE,
    );
    assert_eq!(
        band[0].round,
        [true, true, false, false],
        "no seam rounded mid-block"
    );
    assert!(band.iter().all(|r| !r.mark));
}

/// A mark sitting on a band is its own layer: the band rounds as if it were
/// not there, and the mark splits it into runs only where it covers it.
#[test]
fn a_mark_on_a_band_does_not_square_the_band() {
    let mut cells = row_of(1, 0, 8, A);
    cells.extend(row_of(2, 0, 8, A));
    cells.push(CellView {
        mark: true,
        ..cell(0, 0, 'x', B)
    });
    let got = runs(&cells, 10, 4, PAGE);
    let top = got.iter().find(|r| r.row == 1 && r.col == 0).unwrap();
    assert!(
        top.round[0],
        "the band's corner under row 0's mark still rounds: {got:?}"
    );
}

use super::*;

const TINT: (u8, u8, u8) = (200, 200, 204);
const PAGE: (u8, u8, u8) = (236, 236, 240);
const BADGE: (u8, u8, u8) = (90, 90, 160);

/// Cells of `rows`: `#` is a tinted cell, `B` a badge on a ground of its
/// own, anything else stands on the page.
fn grid(rows: &[&str]) -> Vec<CellView> {
    let mut v = Vec::new();
    for (r, line) in rows.iter().enumerate() {
        for (c, ch) in line.chars().enumerate() {
            let bg = match ch {
                '#' => TINT,
                'B' => BADGE,
                _ => PAGE,
            };
            v.push(CellView {
                col: c as u16,
                row: r as u16,
                c: ch,
                bg,
                ..Default::default()
            });
        }
    }
    v
}

const CHAT: &[&str] = &[
    " prose with a ######### chip   ",
    " and a ########## chip under it",
    " ##########BB## ",
    " ############## ",
    " ############## ",
    " more prose     ",
    " ############## ",
];

/// A block is two or more rows tinted from one column; the badge on its
/// first row does not break it, and a block's last row may be the pane's.
#[test]
fn a_code_block_is_rows_tinted_from_one_column() {
    let f = fields(&grid(CHAT), TINT);
    assert_eq!(
        f,
        vec![Box4 {
            c0: 1,
            r0: 2,
            c1: 14,
            r1: 4
        }]
    );
}

/// Chips in two sentences running are not a block (they start in
/// different columns), and one tinted row alone is not one either.
#[test]
fn chips_and_single_rows_are_left_as_they_are() {
    let f = fields(&grid(&CHAT[..2]), TINT);
    assert!(f.is_empty(), "{f:?}");
    assert!(fields(&grid(&CHAT[5..]), TINT).is_empty());
}

/// Two chips on a line starting where the block does are mostly page
/// between them: not a row of a block.
#[test]
fn a_row_that_is_mostly_page_is_not_a_block_s() {
    let rows = [
        " ########            ######## ",
        " ############################ ",
    ];
    assert!(fields(&grid(&rows), TINT).is_empty());
}

/// The tint leaves the block and only the block: the chips keep theirs and
/// the badge keeps its own ground.
#[test]
fn clearing_takes_the_tint_off_the_block_alone() {
    let mut cells = grid(CHAT);
    let f = fields(&cells, TINT);
    clear(&mut cells, &f, TINT, PAGE);
    let tinted = |row: u16| {
        cells
            .iter()
            .filter(|c| c.row == row && c.bg == TINT)
            .count()
    };
    assert_eq!((tinted(2), tinted(3), tinted(4)), (0, 0, 0));
    assert_eq!((tinted(0), tinted(1), tinted(6)), (9, 10, 14));
    assert_eq!(cells.iter().filter(|c| c.bg == BADGE).count(), 2);
}

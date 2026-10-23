//! The /disk header counts in words that agree with the number.

use crate::diskpane::DiskPane;

fn header(p: &DiskPane) -> String {
    let mut v: Vec<_> = p.cells(80, 20).into_iter().filter(|c| c.row == 0).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect()
}

#[test]
fn a_folder_holding_one_thing_holds_one_entry() {
    let _g = crate::app::theme_test_guard();
    let mut p = DiskPane::new(std::path::PathBuf::from("/tmp/one"));
    p.set_children_for_test(&[("only", 4_096, false)], 0);
    let row = header(&p);
    assert!(row.contains("in 1 entry"), "{row:?}");
    assert!(!row.contains("entries"), "{row:?}");
    p.set_children_for_test(&[("a", 4_096, false), ("b", 1, false)], 0);
    assert!(header(&p).contains("in 2 entries"), "{:?}", header(&p));
}

/// Every label cell lies inside its own tile: a floored fractional top put
/// `.git`'s name half on the tile above it.
#[test]
fn a_label_never_sits_on_the_tile_above() {
    let _g = crate::app::theme_test_guard();
    let mut p = DiskPane::new(std::path::PathBuf::from("/tmp/map"));
    let kids = [
        ("target", 4_200_000_000, true),
        ("crates", 774_000_000, true),
        (".git", 383_000_000, true),
        ("docs", 90_000_000, true),
        ("x", 30_000_000, false),
    ];
    p.set_children_for_test(&kids, 0);
    for (cols, rows) in [(50u16, 20u16), (47, 17), (60, 23), (80, 30), (33, 13)] {
        let map = crate::diskwalk::tiles(&p.children, cols, rows);
        let cells = p.cells(cols, rows);
        for tile in &map {
            let name = kids[tile.index].0;
            let top = tile.y.ceil() as u16;
            // The name's first letter, if drawn, is on a row inside the tile.
            if let Some(c) = cells
                .iter()
                .find(|c| c.row == top && c.col == tile.x as u16 + 1)
            {
                if name.starts_with(c.c) {
                    assert!(
                        f32::from(c.row) >= tile.y
                            && f32::from(c.row) + 1.0 <= tile.y + tile.h + 0.01,
                        "{cols}x{rows} {name}: row {} outside {:?}",
                        c.row,
                        (tile.y, tile.h)
                    );
                }
            }
            // …and never on the row the old floor would have used, when that
            // row belongs to the tile above.
            if tile.y.fract() > 0.01 {
                let floor = tile.y as u16;
                let stray = cells.iter().any(|c| {
                    c.row == floor
                        && c.col == tile.x as u16 + 1
                        && name.starts_with(c.c)
                        && c.c != ' '
                });
                assert!(
                    !stray || floor as f32 >= tile.y,
                    "{cols}x{rows} {name} straddles"
                );
            }
        }
    }
}

/// Every name keeps at least half a cell of air from its tile's left edge.
/// The column was the edge floored plus one, so a tile starting at 10.9
/// wrote its name a tenth of a cell from the edge while its neighbour at
/// 10.1 had most of a cell: `.git` jammed against its border, `target` not.
#[test]
fn a_label_keeps_half_a_cell_from_its_tiles_edge() {
    let _g = crate::app::theme_test_guard();
    let mut p = DiskPane::new(std::path::PathBuf::from("/tmp/air"));
    let kids = [
        ("target", 4_200_000_000, true),
        ("crates", 774_000_000, true),
        (".git", 383_000_000, true),
        ("vendor", 160_000_000, true),
        ("docs", 90_000_000, true),
    ];
    p.set_children_for_test(&kids, 0);
    let mut seen = 0;
    for cols in 40u16..=90 {
        let rows = 24;
        let map = crate::diskwalk::tiles(&p.children, cols, rows);
        let cells = p.cells(cols, rows);
        for tile in &map {
            let first = kids[tile.index].0.chars().next().unwrap();
            let top = tile.y.ceil() as u16;
            let lo = tile.x.floor() as u16;
            let hi = (tile.x + tile.w) as u16;
            let at = cells
                .iter()
                .filter(|c| c.row == top && (lo..hi).contains(&c.col) && c.c != ' ')
                .min_by_key(|c| c.col);
            if let Some(c) = at.filter(|c| c.c == first) {
                seen += 1;
                assert!(
                    f32::from(c.col) >= tile.x + 0.5 - 0.001,
                    "{cols} cols: {} at col {} in a tile from {}",
                    kids[tile.index].0,
                    c.col,
                    tile.x
                );
            }
        }
    }
    assert!(seen > 40, "the labels were found ({seen})");
}

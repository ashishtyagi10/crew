use super::*;

type CellTuple = (u16, u16, char, (u8, u8, u8), (u8, u8, u8));

fn tuples(cells: &[CellView]) -> Vec<CellTuple> {
    cells
        .iter()
        .map(|c| (c.col, c.row, c.c, c.fg, c.bg))
        .collect()
}

#[test]
fn is_land_reads_the_bitmap() {
    assert!(!is_land(0, 0), "north pole band is all sea");
    assert!(is_land(10, 10), "row10 col10 should be North America");
    assert!(!is_land(10, 20), "row10 col20 should be the Atlantic gap");
    assert!(!is_land(EARTH_H, 0), "out-of-range row reads as sea");
    assert!(!is_land(0, EARTH_W), "out-of-range col reads as sea");
}

#[test]
fn shade_char_spans_bright_to_dim() {
    assert_eq!(shade_char(&LAND_CHARS, 1.0), '#');
    assert_eq!(shade_char(&LAND_CHARS, 0.0), '=');
    assert_eq!(shade_char(&SEA_CHARS, 1.0), '·');
    assert_eq!(shade_char(&SEA_CHARS, 0.0), ' ');
}

#[test]
fn globe_cells_stay_within_its_box() {
    let mut cells = Vec::new();
    globe(
        &mut cells,
        3,
        5,
        GLOBE_W,
        GLOBE_H,
        0.0,
        (1, 1, 1),
        (2, 2, 2),
        (0, 0, 0),
    );
    assert!(!cells.is_empty());
    assert!(cells
        .iter()
        .all(|c| { c.col >= 5 && c.col < 5 + GLOBE_W && c.row >= 3 && c.row < 3 + GLOBE_H }));
}

#[test]
fn globe_corners_are_outside_the_disc() {
    for (w, h) in [(GLOBE_MIN_W, GLOBE_MIN_H), (GLOBE_W, GLOBE_H)] {
        let mut cells = Vec::new();
        globe(&mut cells, 0, 0, w, h, 0.0, (1, 1, 1), (2, 2, 2), (0, 0, 0));
        for (col, row) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
            assert!(
                !cells.iter().any(|c| c.col == col && c.row == row),
                "corner ({col},{row}) of {w}x{h} should be outside the disc"
            );
        }
    }
}

#[test]
fn globe_is_deterministic() {
    let mut a = Vec::new();
    let mut b = Vec::new();
    globe(
        &mut a,
        0,
        0,
        GLOBE_W,
        GLOBE_H,
        1.23,
        (1, 1, 1),
        (2, 2, 2),
        (0, 0, 0),
    );
    globe(
        &mut b,
        0,
        0,
        GLOBE_W,
        GLOBE_H,
        1.23,
        (1, 1, 1),
        (2, 2, 2),
        (0, 0, 0),
    );
    assert_eq!(tuples(&a), tuples(&b));
}

#[test]
fn globe_rotation_changes_the_frame() {
    let mut a = Vec::new();
    let mut b = Vec::new();
    globe(
        &mut a,
        0,
        0,
        GLOBE_W,
        GLOBE_H,
        0.0,
        (1, 1, 1),
        (2, 2, 2),
        (0, 0, 0),
    );
    globe(
        &mut b,
        0,
        0,
        GLOBE_W,
        GLOBE_H,
        0.5,
        (1, 1, 1),
        (2, 2, 2),
        (0, 0, 0),
    );
    assert_ne!(tuples(&a), tuples(&b));
}

#[test]
fn globe_shading_partitions_by_char_and_color() {
    let land = (9, 9, 9);
    let sea = (8, 8, 8);
    let mut cells = Vec::new();
    globe(
        &mut cells,
        0,
        0,
        GLOBE_W,
        GLOBE_H,
        0.7,
        land,
        sea,
        (0, 0, 0),
    );
    assert!(!cells.is_empty());
    for c in &cells {
        let is_land_cell = LAND_CHARS.contains(&c.c);
        let is_sea_cell = SEA_CHARS.contains(&c.c);
        assert!(
            is_land_cell != is_sea_cell,
            "{:?} must be exactly one of land/sea",
            c.c
        );
        if is_land_cell {
            assert_eq!(c.fg, land);
        } else {
            assert_eq!(c.fg, sea);
        }
    }
}

#[test]
fn globe_visible_hemisphere_has_land_and_sea() {
    let land = (9, 9, 9);
    let sea = (8, 8, 8);
    let mut cells = Vec::new();
    globe(
        &mut cells,
        0,
        0,
        GLOBE_W,
        GLOBE_H,
        0.0,
        land,
        sea,
        (0, 0, 0),
    );
    assert!(
        cells.iter().any(|c| c.fg == land),
        "phase 0 hemisphere has no land"
    );
    assert!(
        cells.iter().any(|c| c.fg == sea),
        "phase 0 hemisphere has no sea"
    );
}

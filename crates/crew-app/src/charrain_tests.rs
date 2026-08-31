use super::*;

fn frame(w: u16, h: u16, tick: u64) -> Vec<CellView> {
    let mut cells = Vec::new();
    rain(
        &mut cells,
        3,
        5,
        w,
        h,
        tick,
        (0, 255, 0),
        (0, 60, 0),
        (0, 0, 0),
    );
    cells
}

#[test]
fn stays_inside_the_box_and_is_non_empty() {
    let cells = frame(RAIN_W, RAIN_H, 7);
    assert!(!cells.is_empty(), "default-size rain should emit drops");
    assert!(
        cells
            .iter()
            .all(|c| c.col >= 5 && c.col < 5 + RAIN_W && c.row >= 3 && c.row < 3 + RAIN_H),
        "every cell must stay within the given rect"
    );
}

#[test]
fn head_cells_are_bold_and_brightest() {
    let cells = frame(RAIN_W, RAIN_H, 7);
    let heads: Vec<_> = cells.iter().filter(|c| c.bold).collect();
    assert!(!heads.is_empty(), "each active column has a bold head");
    assert!(
        heads.iter().all(|c| c.fg == (0, 255, 0)),
        "head is head colour"
    );
}

#[test]
fn deterministic_in_tick_but_animates() {
    let same = |t| {
        frame(RAIN_W, RAIN_H, t)
            .iter()
            .map(|c| (c.col, c.row, c.c, c.fg))
            .collect::<Vec<_>>()
    };
    assert_eq!(same(7), same(7), "identical tick → identical frame");
    assert_ne!(same(0), same(20), "frames must change over time");
}

#[test]
fn zero_size_emits_nothing() {
    assert!(frame(0, 10, 3).is_empty() && frame(10, 0, 3).is_empty());
}

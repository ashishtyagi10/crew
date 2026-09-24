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
    let mid = |c: &&&CellView| {
        c.col > 5 + 3
            && c.col < 5 + RAIN_W - 4
            && c.row > 3 + 3
            && c.row < 3 + RAIN_H - 4
            && calm(c.col - 5, c.row - 3, RAIN_W, RAIN_H) == 1.0
    };
    assert!(
        heads.iter().filter(mid).all(|c| c.fg == (0, 255, 0)),
        "mid-field head is head colour"
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

/// A streak of light, not line noise: the head is a glyph, the tail
/// dissolves into dots as it fades, and none of the field is the shouting
/// punctuation (`$#%&\{}`…) that made it read as a corrupted terminal.
#[test]
fn a_streak_dissolves_into_dots() {
    for tick in [0, 7, 31, 90] {
        let cells = frame(RAIN_W, RAIN_H, tick);
        let heads = cells.iter().filter(|c| c.bold);
        assert!(
            heads.clone().all(|c| c.c.is_ascii_alphanumeric()),
            "heads are letters or digits"
        );
        let dots = cells.iter().filter(|c| c.c == '\u{00b7}').count();
        assert!(
            dots * 3 >= cells.len(),
            "a third of a streak is its dotted tail: {dots}/{}",
            cells.len()
        );
        assert!(
            cells.iter().all(|c| !"$#%&\\{}[]()|!?_*".contains(c.c)),
            "noise glyphs in the field"
        );
    }
}

/// Mostly page: the field is a few falling streaks, not a wall of glyphs.
#[test]
fn the_field_is_mostly_page() {
    let area = usize::from(RAIN_W) * usize::from(RAIN_H);
    for tick in [0, 7, 31, 90] {
        let lit = frame(RAIN_W, RAIN_H, tick).len();
        assert!(lit * 4 <= area, "tick {tick}: {lit} of {area} cells lit");
    }
}

/// The field is bounded by light, not a ruled box: a glyph at the edge of
/// the rect burns fainter than the same glyph mid-field, so the rain fades
/// out into the page instead of stopping at a line.
#[test]
fn the_field_fades_toward_its_edges() {
    let (bright, dim) = ((0, 255, 0), (0, 60, 0));
    let heads = |cells: &[CellView], edge: bool| -> Vec<u8> {
        cells
            .iter()
            .filter(|c| c.bold && ((c.col == 5 || c.col == 5 + RAIN_W - 1) == edge))
            .map(|c| c.fg.1)
            .collect()
    };
    let mut edge = Vec::new();
    let mut mid = Vec::new();
    for tick in 0..200 {
        let mut cells = Vec::new();
        rain(
            &mut cells,
            3,
            5,
            RAIN_W,
            RAIN_H,
            tick,
            bright,
            dim,
            (0, 0, 0),
        );
        edge.extend(heads(&cells, true));
        mid.extend(heads(&cells, false));
    }
    let max = |v: &[u8]| v.iter().copied().max().unwrap_or(0);
    assert!(!edge.is_empty(), "edge columns rain too");
    assert!(
        max(&edge) < max(&mid),
        "edge {} vs mid {}",
        max(&edge),
        max(&mid)
    );
}

/// The centre, where the name sits, is calm at every tick: no glyph within
/// the calm's inner ellipse, and the field is still full at the edges.
#[test]
fn the_centre_is_calm() {
    for tick in 0..60 {
        let cells = frame(RAIN_W, RAIN_H, tick);
        let (cx, cy) = (5 + RAIN_W / 2, 3 + RAIN_H / 2);
        assert!(
            cells
                .iter()
                .all(|c| c.col.abs_diff(cx) > 8 || c.row.abs_diff(cy) > 1),
            "tick {tick}: a glyph by the name"
        );
    }
    assert_eq!(
        calm(0, 0, RAIN_W, RAIN_H),
        1.0,
        "the corners keep all their light"
    );
}

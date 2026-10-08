use super::*;

/// A `cols`×`rows` frame of plain rules with rounded corners.
fn frame(cols: u16, rows: u16) -> Vec<CellView> {
    let (r, b) = (cols - 1, rows - 1);
    let mut v = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let c = match (col, row) {
                (0, 0) => '\u{256D}',
                (c, 0) if c == r => '\u{256E}',
                (0, y) if y == b => '\u{2570}',
                (c, y) if c == r && y == b => '\u{256F}',
                _ if row == 0 || row == b => RULE_H,
                _ if col == 0 || col == r => RULE_V,
                _ => ' ',
            };
            v.push(CellView {
                col,
                row,
                c,
                ..Default::default()
            });
        }
    }
    v
}

/// Cell sizes crew actually draws at: 13 px at 1x and 2x, a large face, and
/// a tight one.
const CELLS: [(f32, f32); 4] = [(8.4, 18.0), (16.8, 36.0), (11.0, 24.5), (7.0, 15.0)];

#[test]
fn the_radius_is_card_scale_and_stays_off_the_text() {
    for (cw, ch) in CELLS {
        let r = radius(cw, ch);
        // Well past what one cell's glyph could bend (half a cell).
        assert!(r >= cw, "{cw}x{ch}: radius {r} is not card scale");
        let (t, lx, ly) = stroke(cw, ch);
        let near_x = (lx + t / 2.0).min(cw.round() - lx - t / 2.0);
        let near_y = (ly + t / 2.0).min(ch.round() - ly - t / 2.0);
        // Its arc ends inside the first borrowed cell…
        assert!(r < near_x + cw.round(), "{cw}x{ch}: {r} overruns the arm");
        // …and bows no nearer the first cell of content than a pixel.
        assert!(
            bow(r, near_x) + t / 2.0 <= near_y - 1.0,
            "{cw}x{ch}: into the text"
        );
    }
}

#[test]
fn every_corner_of_a_plain_frame_rounds() {
    for (cw, ch) in CELLS {
        let found = find(&frame(10, 5), cw, ch);
        let at: Vec<(u16, u16)> = found.iter().map(|k| (k.col, k.row)).collect();
        assert_eq!(at, vec![(0, 0), (9, 0), (0, 4), (9, 4)], "{cw}x{ch}");
        assert!(found.iter().all(|k| k.nx >= 1 && k.ny >= 1));
    }
}

#[test]
fn a_legend_against_the_corner_keeps_its_glyph() {
    // `╭x…`: the arc would run through the legend's first letter.
    let mut cells = frame(10, 5);
    cells
        .iter_mut()
        .find(|c| c.col == 1 && c.row == 0)
        .unwrap()
        .c = 'x';
    let found = find(&cells, 8.4, 18.0);
    assert!(found.iter().all(|k| (k.col, k.row) != (0, 0)), "{found:?}");
    assert_eq!(found.len(), 3);
}

#[test]
fn corners_too_close_to_bend_apart_keep_their_glyphs() {
    // Three columns: the two top arcs would meet over the one `─` between.
    for (cw, ch) in CELLS {
        assert!(find(&frame(3, 4), cw, ch).is_empty(), "{cw}x{ch}");
    }
}

#[test]
fn a_one_row_card_rounds_all_four_corners() {
    // The input bar, a one-line toast: top and bottom corners share the one
    // `│` between them, each arc done inside its own half.
    for (cw, ch) in CELLS {
        assert_eq!(find(&frame(12, 3), cw, ch).len(), 4, "{cw}x{ch}");
    }
}

#[test]
fn blanking_clears_exactly_the_arcs_cells() {
    let pane = PaneScene {
        cells: frame(10, 5),
        ..Default::default()
    };
    let found = find(&pane.cells, 8.4, 18.0);
    let out = blanked(&pane, &found);
    let gone: HashSet<(i32, i32)> = found.iter().flat_map(Corner::cells).collect();
    for (a, b) in pane.cells.iter().zip(&out.cells) {
        let p = (i32::from(a.col), i32::from(a.row));
        if gone.contains(&p) {
            assert_eq!(b.c, ' ', "{p:?}");
        } else {
            assert_eq!(b.c, a.c, "{p:?} should be untouched");
        }
    }
    // The four corners' glyphs are gone, and the middles of the rules stay.
    assert!(out.cells.iter().all(|c| arms(c.c).is_none()));
    assert!(out.cells.iter().any(|c| c.c == RULE_H));
}

#[test]
fn square_corners_round_too() {
    let mut cells = frame(10, 5);
    for c in &mut cells {
        c.c = match c.c {
            '\u{256D}' => '\u{250C}',
            '\u{256E}' => '\u{2510}',
            '\u{2570}' => '\u{2514}',
            '\u{256F}' => '\u{2518}',
            x => x,
        };
    }
    assert_eq!(find(&cells, 8.4, 18.0).len(), 4);
}

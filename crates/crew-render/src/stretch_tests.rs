use super::*;

const CW: f32 = 10.0;
const CH: f32 = 20.0;

/// A `cols`×`rows` rounded frame with a title, as a card scene lays it out.
fn frame(cols: u16, rows: u16) -> Vec<CellView> {
    let mut out = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let c = match (col, row) {
                (0, 0) => '╭',
                (c, 0) if c == cols - 1 => '╮',
                (0, r) if r == rows - 1 => '╰',
                (c, r) if c == cols - 1 && r == rows - 1 => '╯',
                (2, 0) => 'T',
                (_, r) if r == 0 || r == rows - 1 => '─',
                (c, _) if c == 0 || c == cols - 1 => '│',
                _ => continue,
            };
            out.push(CellView {
                col,
                row,
                c,
                fg: (1, 2, 3),
                ..Default::default()
            });
        }
    }
    out
}

fn card(w: f32, h: f32, cols: u16, rows: u16) -> PaneScene {
    PaneScene {
        cells: frame(cols, rows),
        x: 100.0,
        y: 50.0,
        w,
        h,
        stretch: true,
        ..Default::default()
    }
}

/// The remainder is what is left between the frame's last cell and the rect.
#[test]
fn a_frame_short_of_its_rect_stretches_by_the_remainder() {
    let s = split(&card(87.0, 133.0, 8, 6), CW, CH).expect("stretches");
    assert_eq!((s.lc, s.lr), (7, 5));
    assert_eq!((s.sx, s.sy), (7.0, 13.0));
}

/// No flag, no stretch; nothing left over, no stretch; a frame that overruns
/// its rect mid-glide draws as it always did.
#[test]
fn only_a_frame_with_room_stretches() {
    let mut plain = card(87.0, 133.0, 8, 6);
    plain.stretch = false;
    assert_eq!(split(&plain, CW, CH), None);
    assert_eq!(split(&card(80.0, 120.0, 8, 6), CW, CH), None);
    assert_eq!(split(&card(60.0, 100.0, 8, 6), CW, CH), None);
}

/// The last column and row move out to the rect's far edges, re-indexed to
/// their own grids; the body keeps its origin.
#[test]
fn the_far_column_and_row_move_out_to_the_rect_edges() {
    let pane = card(87.0, 133.0, 8, 6);
    let s = split(&pane, CW, CH).unwrap();
    let p = parts(&pane, &s, CW, CH);
    assert_eq!(p.len(), 4, "body, right column, bottom row, corner");
    assert_eq!((p[0].x, p[0].y, p[0].w, p[0].h), (100.0, 50.0, 70.0, 100.0));
    assert_eq!((p[1].x, p[1].y), (100.0 + 70.0 + 7.0, 50.0));
    assert_eq!((p[2].x, p[2].y), (100.0, 50.0 + 100.0 + 13.0));
    assert_eq!(
        (p[3].x + p[3].w, p[3].y + p[3].h),
        (187.0, 183.0),
        "the corner ends at the rect's corner"
    );
    assert!(p[1].cells.iter().all(|c| c.col == 0), "re-indexed");
    assert!(p[3]
        .cells
        .iter()
        .any(|c| c.c == '╯' && (c.col, c.row) == (0, 0)));
    let n: usize = p.iter().map(|p| p.cells.len()).sum();
    assert_eq!(n, pane.cells.len(), "every cell lands in exactly one part");
}

/// The top and bottom rules continue across the horizontal stretch, the side
/// rules across the vertical one — and nothing else is bridged.
#[test]
fn every_rule_crossing_the_seam_is_bridged() {
    let pane = card(87.0, 133.0, 8, 6);
    let s = split(&pane, CW, CH).unwrap();
    let b = bridges(&pane, &s, CW, CH);
    let across: Vec<_> = b.iter().filter(|q| q.2 > q.3).collect();
    let down: Vec<_> = b.iter().filter(|q| q.3 > q.2).collect();
    assert_eq!(across.len(), 2, "top and bottom rule");
    assert_eq!(down.len(), 2, "left and right rule");
    // Across the seam, one pixel into the glyph either side.
    assert_eq!((across[0].0, across[0].0 + across[0].2), (169.0, 178.0));
    // The bottom rule's bridge sits in the moved-out bottom row.
    assert!(across[1].1 > 50.0 + 100.0 + 13.0);
    // The right rule's bridge sits in the moved-out column.
    assert!(down[1].0 > 100.0 + 70.0 + 7.0);
    assert!(
        b.iter().all(|q| q.4 == (1, 2, 3)),
        "in the rule's own colour"
    );
}

/// A legend touching the seam has no arm across it: the rule stops there,
/// as it does beside any legend.
#[test]
fn a_legend_at_the_seam_is_not_bridged() {
    let mut pane = card(87.0, 133.0, 8, 6);
    for c in pane.cells.iter_mut().filter(|c| (c.col, c.row) == (6, 0)) {
        c.c = 'x';
    }
    let s = split(&pane, CW, CH).unwrap();
    let across = bridges(&pane, &s, CW, CH)
        .into_iter()
        .filter(|q| q.2 > q.3)
        .count();
    assert_eq!(across, 1, "only the bottom rule");
}

/// Paint in the last column rides out with it; paint spanning the seam
/// stretches across it; paint before it stays put.
#[test]
fn paint_lands_where_the_stretched_frame_is() {
    let mut pane = card(87.0, 133.0, 8, 6);
    pane.paint = vec![
        Paint::solid(7.25, 1.0, 0.5, 2.0, (0, 0, 0)),
        Paint::solid(1.0, 5.5, 7.0, 0.2, (0, 0, 0)),
        Paint::solid(1.0, 1.0, 2.0, 1.0, (0, 0, 0)),
    ];
    let s = split(&pane, CW, CH).unwrap();
    let body = &parts(&pane, &s, CW, CH)[0];
    let p = &body.paint;
    assert_eq!(
        p[0].x * CW,
        72.5 + 7.0,
        "the thumb moved out with its column"
    );
    assert_eq!(
        (p[1].w * CW, p[1].y * CH),
        (77.0, 110.0 + 13.0),
        "the bar spans the seam, on the moved row"
    );
    assert_eq!((p[2].x, p[2].w), (1.0, 2.0), "untouched");
}

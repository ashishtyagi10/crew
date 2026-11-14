use super::*;

fn row(out: &mut Vec<CellView>, r: u16, s: &str) {
    for (i, c) in s.chars().enumerate() {
        out.push(CellView {
            col: i as u16,
            row: r,
            c,
            ..Default::default()
        });
    }
}

fn text(cells: &[CellView], r: u16) -> String {
    let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == r).collect();
    v.sort_by_key(|c| c.col);
    let s: String = v.iter().map(|c| c.c).collect();
    s.trim_end().to_string()
}

/// Two sections, a PANES heading, and two tab cards on adjacent rows.
fn nav() -> Vec<CellView> {
    let mut v = Vec::new();
    for (r, s) in [
        " \u{2500} TIME \u{2500}\u{2500}\u{2500} ",
        "   16:45     ",
        "",
        " \u{2500} LOAD \u{2500}\u{2500}\u{2500} ",
        "   1.4 2.4   ",
        "",
        " \u{2500} PANES \u{2500}\u{2500} ",
        " \u{256d} 1 \u{b7} a \u{2500}\u{2500}\u{256e} ",
        " \u{2502} 1 far   \u{2502} ",
        " \u{2570}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{256f} ",
        " \u{256d} 2 \u{b7} b \u{2500}\u{2500}\u{256e} ",
        " \u{2502} 2 a\u{2500}b   \u{2502} ",
        " \u{2570}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{256f} ",
    ]
    .iter()
    .enumerate()
    {
        row(&mut v, r as u16, s);
    }
    v
}

/// Every section is a tile from its rule to its last row, each tab card is
/// one, and the heading over the tab cards is not.
#[test]
fn a_tile_per_section_and_per_tab_card() {
    let t: Vec<(u16, u16, bool)> = tiles(&nav())
        .iter()
        .map(|t| (t.top, t.bottom, t.tab))
        .collect();
    assert_eq!(
        t,
        [(0, 1, false), (3, 4, false), (7, 9, true), (10, 12, true)]
    );
}

/// A flat pane list is the last section: its tile ends on the last row
/// with ink, having no rule after it to end before.
#[test]
fn the_last_section_ends_on_its_last_row() {
    let mut v = Vec::new();
    row(&mut v, 0, " \u{2500} PANES 2 \u{2500} ");
    row(&mut v, 1, "   1 far     ");
    row(&mut v, 2, "   2 dash    ");
    row(&mut v, 3, "             ");
    assert_eq!(
        tiles(&v),
        [Tile {
            top: 0,
            bottom: 2,
            tab: false,
            lifted: false
        }]
    );
}

/// The strokes go and the words stay, a rule inside a pane's name included.
#[test]
fn the_rules_are_blanked_and_the_words_kept() {
    let mut v = nav();
    let t = tiles(&v);
    unruled(&mut v, &t);
    assert_eq!(text(&v, 0), "   TIME");
    assert_eq!(text(&v, 6), "   PANES");
    assert_eq!(text(&v, 7), "   1 \u{b7} a");
    assert_eq!(text(&v, 9), "");
    assert_eq!(text(&v, 11), "   2 a\u{2500}b");
}

/// Two sections one gap row apart, and two tab cards on adjacent rows,
/// both end up `gap` px apart: the gutter the panes keep.
#[test]
fn neighbouring_tiles_stand_one_gutter_apart() {
    let (ch, gap) = (17.0, 8.0);
    let rect = Rect {
        x: 4.0,
        y: 10.0,
        w: 200.0,
        h: 600.0,
    };
    let t = tiles(&nav());
    let r: Vec<Rect> = t.iter().map(|t| rect_of(t, rect, ch, gap)).collect();
    for pair in [(0, 1), (2, 3)] {
        let (a, b) = (r[pair.0], r[pair.1]);
        assert!((b.y - (a.y + a.h) - gap).abs() < 1e-3, "{a:?} {b:?}");
    }
    assert!(r.iter().all(|r| r.x == rect.x && r.w == rect.w));
    // The first tile keeps off the nav's legend row above it.
    assert_eq!(r[0].y, rect.y + ch);
}

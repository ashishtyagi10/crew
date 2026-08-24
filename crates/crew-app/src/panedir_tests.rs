use super::{step, Dir};
use crate::layout::Rect;

fn r(x: f32, y: f32) -> Rect {
    Rect {
        x,
        y,
        w: 100.0,
        h: 100.0,
    }
}

/// A 2×2 grid laid out the way `pane_rects_at` lays one out: two columns,
/// each split into two rows. Indices run down the first column, then the
/// second — which is exactly why index order is not spatial order.
fn quad() -> Vec<(usize, Rect)> {
    vec![
        (0, r(0.0, 0.0)),
        (1, r(0.0, 100.0)),
        (2, r(100.0, 0.0)),
        (3, r(100.0, 100.0)),
    ]
}

#[test]
fn moves_to_the_neighbour_on_each_side() {
    let g = quad();
    assert_eq!(step(&g, 0, Dir::Down), Some(1));
    assert_eq!(step(&g, 0, Dir::Right), Some(2));
    assert_eq!(step(&g, 3, Dir::Up), Some(2));
    assert_eq!(step(&g, 3, Dir::Left), Some(1));
}

#[test]
fn stops_at_the_edge_instead_of_wrapping() {
    let g = quad();
    assert_eq!(step(&g, 0, Dir::Up), None);
    assert_eq!(step(&g, 0, Dir::Left), None);
    assert_eq!(step(&g, 3, Dir::Down), None);
    assert_eq!(step(&g, 3, Dir::Right), None);
}

#[test]
fn nearest_on_the_axis_wins_over_a_farther_aligned_pane() {
    // Three columns in one row: from the left column, Right must land on the
    // middle one, never skip to the far one.
    let g = vec![(0, r(0.0, 0.0)), (1, r(100.0, 0.0)), (2, r(200.0, 0.0))];
    assert_eq!(step(&g, 0, Dir::Right), Some(1));
    assert_eq!(step(&g, 2, Dir::Left), Some(1));
}

#[test]
fn ties_on_the_axis_break_toward_the_nearer_row() {
    // One tall pane on the left, two stacked on the right: moving Right from
    // the tall pane's upper half picks the upper of the two.
    let tall = Rect {
        x: 0.0,
        y: 0.0,
        w: 100.0,
        h: 60.0,
    };
    let g = vec![(0, tall), (1, r(100.0, 0.0)), (2, r(100.0, 200.0))];
    assert_eq!(step(&g, 0, Dir::Right), Some(1));
}

#[test]
fn a_pane_not_on_screen_has_no_neighbours() {
    // `from` missing from the placement (minimized past the strip's overflow)
    // yields None rather than picking an arbitrary tile.
    assert_eq!(step(&quad(), 9, Dir::Right), None);
}

#[test]
fn a_lone_pane_has_nowhere_to_go() {
    let g = vec![(0, r(0.0, 0.0))];
    for d in [Dir::Left, Dir::Right, Dir::Up, Dir::Down] {
        assert_eq!(step(&g, 0, d), None);
    }
}

#[test]
fn the_minimized_strip_is_reachable_downward() {
    // A full tile above, a strip thumbnail below: Down reaches the strip and
    // Up comes back out of it.
    let g = vec![
        (
            0,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 200.0,
                h: 300.0,
            },
        ),
        (
            1,
            Rect {
                x: 0.0,
                y: 300.0,
                w: 200.0,
                h: 40.0,
            },
        ),
    ];
    assert_eq!(step(&g, 0, Dir::Down), Some(1));
    assert_eq!(step(&g, 1, Dir::Up), Some(0));
}

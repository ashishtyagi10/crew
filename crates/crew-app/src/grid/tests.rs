use super::*;

#[test]
fn add_orders_most_recent_first() {
    let mut g = GridLayout::new();
    g.add(0);
    g.add(1);
    g.add(2);
    assert_eq!(g.full(), &[2, 1, 0]);
    assert!(g.minimized().is_empty());
    assert_eq!(g.len(), 3);
}

#[test]
fn add_is_idempotent_and_promotes() {
    let mut g = GridLayout::new();
    g.add(0);
    g.add(1);
    g.add(0); // re-add existing -> front, no duplicate
    assert_eq!(g.full(), &[0, 1]);
    assert_eq!(g.len(), 2);
}

#[test]
fn seventh_tile_minimizes_least_recent() {
    let mut g = GridLayout::new();
    for idx in 0..7 {
        g.add(idx);
    }
    assert_eq!(g.full(), &[6, 5, 4, 3, 2, 1]);
    assert_eq!(g.minimized(), &[0]);
}

#[test]
fn touch_promotes_into_full_set() {
    let mut g = GridLayout::new();
    for idx in 0..7 {
        g.add(idx);
    }
    g.touch(0); // 0 was minimized; promote it, demote current LRU (1)
    assert_eq!(g.full()[0], 0);
    assert_eq!(g.minimized(), &[1]);
}

#[test]
fn touch_absent_is_noop() {
    let mut g = GridLayout::new();
    g.add(0);
    g.touch(99);
    assert_eq!(g.full(), &[0]);
}

#[test]
fn on_close_removes_and_shifts_indices() {
    // Panes [0,1,2,3] added; close index 1. After Vec::remove(1), old panes
    // 2,3 become indices 1,2. The LRU must reflect that.
    let mut g = GridLayout::new();
    for idx in 0..4 {
        g.add(idx); // order front->back: 3,2,1,0
    }
    g.on_close(1);
    // 1 is gone; 2->1 and 3->2. Order was [3,2,1,0] -> drop 1 -> [3,2,0]
    // -> shift (>1 decremented): 3->2, 2->1, 0 stays -> [2,1,0].
    assert_eq!(g.full(), &[2, 1, 0]);
    assert_eq!(g.len(), 3);
}

#[test]
fn empty_state() {
    let g = GridLayout::new();
    assert!(g.is_empty());
    assert_eq!(g.len(), 0);
    assert!(g.full().is_empty());
    assert!(g.minimized().is_empty());
}

use crate::layout::Rect;

fn content() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    }
}

#[test]
fn compose_empty_is_empty() {
    let g = GridLayout::new();
    let out = compose_grid(content(), &g, 16.0, 8.0);
    assert!(out.full.is_empty());
    assert!(out.minimized.is_empty());
}

#[test]
fn compose_no_minimized_uses_full_height() {
    let mut g = GridLayout::new();
    g.add(0);
    let out = compose_grid(content(), &g, 16.0, 8.0);
    assert_eq!(out.full.len(), 1);
    assert!(out.minimized.is_empty());
    assert_eq!(out.full[0].0, 0); // index preserved
                                  // One tile, no strip: height is content minus the grid gap insets only.
    assert!(out.full[0].1.h > 560.0, "tile should use ~full height");
}

#[test]
fn compose_reserves_strip_when_minimized_present() {
    let mut g = GridLayout::new();
    for idx in 0..7 {
        g.add(idx);
    }
    let out = compose_grid(content(), &g, 16.0, 8.0);
    assert_eq!(out.full.len(), 6);
    assert_eq!(out.minimized.len(), 1);
    // The full grid sits entirely above the strip.
    let full_bottom = out
        .full
        .iter()
        .map(|(_, r)| r.y + r.h)
        .fold(0.0_f32, f32::max);
    let strip_top = out.minimized[0].1.y;
    assert!(full_bottom <= strip_top + 0.5, "full grid overlaps strip");
    // Strip sits at the bottom of the content area.
    let strip_bottom = out.minimized[0].1.y + out.minimized[0].1.h;
    assert!(strip_bottom <= 600.0 + 0.5);
    assert!(out.minimized[0].1.y > 600.0 - (4.0 * 16.0 + 2.0 * 8.0) - 1.0);
}

#[test]
fn compose_full_indices_sorted_stable() {
    // LRU order is 2,1,0 (most-recent first), but display order is stable by
    // pane index so focusing never moves a tile.
    let mut g = GridLayout::new();
    g.add(0);
    g.add(1);
    g.add(2);
    let out = compose_grid(content(), &g, 16.0, 8.0);
    let ids: Vec<usize> = out.full.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, vec![0, 1, 2]);
}

#[test]
fn compose_minimized_indices_sorted_stable() {
    // Even though touch() reorders the LRU, the minimized strip stays in
    // stable index order so thumbnails don't jump.
    let mut g = GridLayout::new();
    for idx in 0..8 {
        g.add(idx); // LRU front->back: 7..0; full = 7,6,5,4,3,2 ; min = 1,0
    }
    let out = compose_grid(content(), &g, 16.0, 8.0);
    let ids: Vec<usize> = out.minimized.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, vec![0, 1]);
}

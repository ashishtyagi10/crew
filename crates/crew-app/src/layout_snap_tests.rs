//! Device-pixel snapping tests: layout output must land on whole physical
//! pixels however fractional the inputs (fractional cell heights, odd areas,
//! Retina logical coordinates), without opening seams between neighbors.
use super::*;

fn integral(v: f32) -> bool {
    v.fract() == 0.0
}

fn assert_on_grid(r: &Rect) {
    assert!(
        integral(r.x) && integral(r.y) && integral(r.x + r.w) && integral(r.y + r.h),
        "rect off the device-pixel grid: {r:?}"
    );
}

#[test]
fn fractional_layout_snaps_every_edge_to_device_pixels() {
    // A fractional content area is the everyday case: compose_grid subtracts a
    // strip of `rows * cell_h` where cell_h is fractional (font 14 → 17.5).
    for r in pane_rects_at(3, 0.3, 0.7, 795.5, 601.3, 8.0) {
        assert_on_grid(&r);
    }
}

#[test]
fn retina_odd_logical_origin_lands_on_a_device_pixel() {
    // Scale 2.0, odd logical origin 101.3 → physical 202.6. Layout runs in
    // physical px, so the snapped origin is the whole device pixel itself.
    let scale = 2.0;
    let r = Rect {
        x: 101.3 * scale,
        y: 33.75 * scale,
        w: 400.2,
        h: 300.9,
    }
    .snapped();
    assert_on_grid(&r);
    // Snapping moves each edge less than a pixel — it sharpens, never shifts.
    assert!((r.x - 202.6).abs() < 1.0 && (r.y - 67.5).abs() < 1.0);
}

#[test]
fn integral_rects_snap_to_themselves() {
    let r = Rect {
        x: 208.0,
        y: 0.0,
        w: 792.0,
        h: 740.0,
    };
    assert_eq!(r.snapped(), r);
}

#[test]
fn zero_gap_neighbors_share_the_snapped_edge() {
    // 801/2 = 400.5: both tiles see the same pre-snap boundary, so both must
    // snap it to the same pixel — no 1px gap or overlap between them.
    let r = pane_rects_at(2, 0.0, 0.0, 801.0, 601.0, 0.0);
    assert_eq!(r[0].x + r[0].w, r[1].x, "seam must stay shared");
    assert_on_grid(&r[0]);
    assert_on_grid(&r[1]);
    assert_eq!(
        r[1].x + r[1].w,
        801.0,
        "right edge stays on the outer bound"
    );
}

#[test]
fn minimized_strip_rects_land_on_device_pixels() {
    use crate::grid::{compose_grid, GridLayout};
    let mut g = GridLayout::new();
    for idx in 0..7 {
        g.add(idx); // 6 full tiles + 1 minimized thumbnail
    }
    let content = Rect {
        x: 208.0,
        y: 0.0,
        w: 795.5,
        h: 601.3,
    };
    // Fractional cell height: the strip offset (rows * 16.3) is fractional.
    let out = compose_grid(content, &g, 16.3, 8.0);
    for (_, r) in out.full.iter().chain(out.minimized.iter()) {
        assert_on_grid(r);
    }
}

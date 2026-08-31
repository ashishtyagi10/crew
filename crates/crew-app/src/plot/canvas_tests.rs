use super::Canvas;

/// Total painted area, in square units — the invariant every shape test
/// below leans on. Alpha counts: a half-covered edge pixel is half a
/// pixel of area, which is what anti-aliasing means.
fn painted_area(c: &Canvas) -> f32 {
    c.paint()
        .iter()
        .map(|p| p.w * p.h * c.row_units() * p.alpha)
        .sum()
}

#[test]
fn user_space_is_square_so_a_circle_is_round() {
    // Two cells across, one row down, at a 2:1 cell — the canvas is 2
    // units wide and 2 units tall, not 2 × 1.
    let c = Canvas::new(2, 1, 2.0);
    let (w, h) = c.size();
    assert_eq!((w, h), (2.0, 2.0));
}

#[test]
fn a_filled_rectangle_covers_exactly_its_area() {
    let mut c = Canvas::new(4, 2, 2.0);
    c.rect(1.0, 1.0, 2.0, 1.5, (255, 0, 0), 1.0);
    let a = painted_area(&c);
    assert!((a - 3.0).abs() < 0.02, "3 square units painted, got {a}");
}

#[test]
fn a_disc_covers_pi_r_squared_with_anti_aliased_edges() {
    let mut c = Canvas::with_sub(6, 3, 2.0, 8);
    let (cx, cy, r) = (3.0, 3.0, 2.0);
    c.fill(
        (cx - r, cy - r, 2.0 * r, 2.0 * r),
        (0, 255, 0),
        1.0,
        |x, y| (x - cx).powi(2) + (y - cy).powi(2) <= r * r,
    );
    let a = painted_area(&c);
    let want = std::f32::consts::PI * r * r;
    assert!(
        (a - want).abs() / want < 0.02,
        "disc area {a} within 2% of {want}"
    );
    // The edge is graded, not binary: a hard-edged raster would put every
    // pixel at alpha 1.0 and stair-step at these sizes.
    let graded = c.paint().iter().filter(|p| p.alpha < 0.95).count();
    assert!(graded > 8, "anti-aliased edge pixels: {graded}");
}

/// The point of the distance path: an edge that lands between two canvas
/// pixels comes out as a partial one, and a mark thinner than a pixel
/// comes out *grey* rather than missing. The sampled path can do the
/// first now — an edge pixel is re-sampled at 7×7 for fifty levels — but
/// never the second: a mark that falls between the coarse samples is
/// never refined, because nothing noticed it was there.
#[test]
fn a_distance_fill_grades_edges_the_sample_grid_would_snap_or_miss() {
    let (cx, cy, r) = (3.0, 3.0, 2.0);
    let bbox = (cx - r, cy - r, 2.0 * r, 2.0 * r);
    let disc = move |x: f32, y: f32| super::super::sdf::disc((x, y), (cx, cy), r);
    let levels = |c: &Canvas| {
        let mut steps: Vec<f32> = c
            .paint()
            .iter()
            .map(|p| p.alpha)
            .filter(|a| *a < 0.99)
            .collect();
        steps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        steps.dedup();
        steps.len()
    };

    let mut sdf = Canvas::new(6, 3, 2.0);
    sdf.fill_sdf(bbox, (0, 255, 0), 1.0, disc);
    let a = painted_area(&sdf);
    let want = std::f32::consts::PI * r * r;
    assert!((a - want).abs() / want < 0.02, "disc area {a} vs {want}");

    // The same disc, sampled. This used to land on a handful of levels —
    // nine samples can only ever say ninths — and a near-horizontal edge
    // graded in ninths terraces visibly now that a canvas pixel IS a
    // screen pixel. The edge pixels are refined, so both paths grade.
    let mut sampled = Canvas::new(6, 3, 2.0);
    sampled.fill(bbox, (0, 255, 0), 1.0, move |x, y| disc(x, y) <= 0.0);
    assert!(
        levels(&sampled) > 4,
        "a sampled edge came out in {} levels",
        levels(&sampled)
    );
    assert!(levels(&sdf) > 4, "a distance edge lost its grading");
    // Both describe the same circle to within a pixel of area.
    let sa = painted_area(&sampled);
    assert!(
        (sa - want).abs() / want < 0.06,
        "sampled disc area {sa} vs {want}"
    );
}

/// A hairline thinner than a canvas pixel: the sample grid draws nothing
/// unless it happens to straddle a sample row, the distance field always
/// draws it, dimmed in proportion to how thin it is.
#[test]
fn a_sub_pixel_line_survives_the_distance_path() {
    let thin = |use_sdf: bool| {
        let mut c = Canvas::new(8, 2, 2.0);
        // A tenth of a pixel wide, deliberately placed off the sample rows.
        let (y, half) = (1.507, 0.05 * c.px());
        if use_sdf {
            c.fill_sdf(
                (0.0, y - 0.5, 8.0, 1.0),
                (255, 255, 255),
                1.0,
                move |_, py| (py - y).abs() - half,
            );
        } else {
            c.fill(
                (0.0, y - 0.5, 8.0, 1.0),
                (255, 255, 255),
                1.0,
                move |_, py| (py - y).abs() <= half,
            );
        }
        painted_area(&c)
    };
    assert_eq!(thin(false), 0.0, "the sample grid misses it entirely");
    assert!(thin(true) > 0.0, "the distance field keeps it");
}

#[test]
fn paint_merges_runs_in_both_directions() {
    let mut c = Canvas::new(8, 4, 2.0);
    let (w, h) = c.size();
    c.rect(0.0, 0.0, w, h, (1, 2, 3), 1.0);
    // A solid fill is ONE rectangle — not one per pixel row, and not one
    // per pixel. Without the merge a full-pane chart would push tens of
    // thousands of quads a frame.
    assert_eq!(c.paint().len(), 1);
    let p = c.paint()[0];
    assert!((p.w - 8.0).abs() < 1e-3 && (p.h - 4.0).abs() < 1e-3);
}

#[test]
fn nothing_drawn_paints_nothing() {
    assert!(Canvas::new(10, 4, 2.0).paint().is_empty());
}

#[test]
fn paint_stays_inside_the_canvas_however_far_the_shape_runs() {
    let mut c = Canvas::new(3, 2, 2.0);
    c.rect(-5.0, -5.0, 100.0, 100.0, (7, 7, 7), 1.0);
    for p in c.paint() {
        assert!(p.x >= 0.0 && p.y >= 0.0);
        assert!(p.x + p.w <= 3.0 + 1e-3, "column overflow: {p:?}");
        assert!(p.y + p.h <= 2.0 + 1e-3, "row overflow: {p:?}");
    }
}

#[test]
fn translucent_paint_composites_rather_than_replaces() {
    let mut c = Canvas::new(2, 1, 2.0);
    c.rect(0.0, 0.0, 2.0, 2.0, (0, 0, 0), 1.0);
    c.rect(0.0, 0.0, 2.0, 2.0, (255, 255, 255), 0.5);
    let p = c.paint();
    assert_eq!(p.len(), 1);
    // Half white over black reads mid-grey and fully opaque — the layer
    // below is *covered*, not discarded.
    assert!((p[0].alpha - 1.0).abs() < 1e-3);
    assert!((110..=145).contains(&p[0].color.0), "got {:?}", p[0].color);
}

#[test]
fn a_shaded_fill_varies_across_the_shape() {
    let mut c = Canvas::new(4, 2, 2.0);
    let (w, h) = c.size();
    c.fill_shaded(
        (0.0, 0.0, w, h),
        |_, _| true,
        |_, y| ((255, 255, 255), (y / h).clamp(0.0, 1.0)),
    );
    let alphas: Vec<f32> = c.paint().iter().map(|p| p.alpha).collect();
    let lo = alphas.iter().cloned().fold(f32::MAX, f32::min);
    let hi = alphas.iter().cloned().fold(0.0f32, f32::max);
    assert!(hi > 0.9 && lo < 0.2, "gradient spans alpha {lo}..{hi}");
}

#[test]
fn the_pixel_grid_follows_the_cell_aspect() {
    // A taller cell means more pixel rows for the same cell count, so the
    // shapes drawn in it keep their screen proportions.
    let tall = Canvas::new(4, 2, 3.0);
    let wide = Canvas::new(4, 2, 1.0);
    assert_eq!(tall.size().0, wide.size().0);
    assert!(tall.size().1 > wide.size().1);
    assert_eq!(wide.size().0, 4.0);
    assert_eq!(tall.w, 4 * crate::plot::device::sub());
}

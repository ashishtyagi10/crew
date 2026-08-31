use super::*;

/// A distance field is only worth having if it is *metric*: the value has
/// to be the distance in units, because coverage divides it by the pixel
/// size. A field that is merely correctly-signed anti-aliases wrongly.
#[test]
fn a_discs_field_reads_in_units_not_just_signs() {
    let c = (5.0, 5.0);
    assert!((disc((5.0, 5.0), c, 2.0) + 2.0).abs() < 1e-5, "centre");
    assert!(disc((7.0, 5.0), c, 2.0).abs() < 1e-5, "on the edge");
    assert!((disc((8.5, 5.0), c, 2.0) - 1.5).abs() < 1e-5, "1.5 outside");
}

#[test]
fn a_ring_is_the_band_around_its_radius() {
    let c = (0.0, 0.0);
    assert!(ring((3.0, 0.0), c, 3.0, 0.5) < 0.0, "on the radius");
    assert!(ring((0.0, 0.0), c, 3.0, 0.5) > 0.0, "the hole is outside");
    assert!(ring((9.0, 0.0), c, 3.0, 0.5) > 0.0, "and so is beyond it");
    assert!((ring((3.4, 0.0), c, 3.0, 0.5) + 0.1).abs() < 1e-5);
}

#[test]
fn an_arc_runs_clockwise_from_noon() {
    let c = (0.0, 0.0);
    let quarter = |p| arc(p, c, 3.0, 0.4, 0.0, TAU * 0.25);
    assert!(quarter((0.0, -3.0)) < 0.0, "noon is in it");
    assert!(quarter((3.0, 0.0)) < 0.0, "and three o'clock");
    assert!(quarter((-3.0, 0.0)) > 0.0, "nine o'clock is not");
    assert!(quarter((0.0, 3.0)) > 0.0, "nor six");
}

/// The two ends every gauge lives or dies by: nothing swept still marks
/// where the sweep starts, and everything swept has no seam.
#[test]
fn the_arcs_ends_are_round_and_its_full_sweep_is_seamless() {
    let c = (0.0, 0.0);
    // A hair of a sweep is a dot of the band's own thickness at noon.
    let dot = |p| arc(p, c, 3.0, 0.4, 0.0, 0.001);
    assert!(dot((0.0, -3.0)) < 0.0 && dot((0.3, -3.0)) < 0.0);
    assert!(dot((0.0, 3.0)) > 0.0);
    // A full sweep is the ring — every angle inside, no cap sticking out.
    for i in 0..16 {
        let a = TAU * i as f32 / 16.0;
        let p = polar(c, 3.0, a);
        assert!(arc(p, c, 3.0, 0.4, 0.0, TAU) < 0.0, "angle {a}");
    }
}

/// A slice is cut on its radii: the ends are straight, and a slice of
/// nothing is nothing rather than a dot (which is what [`arc`]'s round
/// caps would leave behind).
#[test]
fn a_sector_is_a_wedge_of_the_band_with_square_ends() {
    let c = (0.0, 0.0);
    let q = |p| sector(p, c, 4.0, 2.0, 0.0, TAU * 0.25);
    assert!(q(polar(c, 3.0, TAU * 0.125)) < 0.0, "mid-slice, mid-band");
    // Its two radii are the boundary, not the inside: a slice ends on
    // them, and the next slice begins.
    assert!(q((0.0, -3.0)).abs() < 1e-5, "noon is the cut");
    assert!(q((3.0, 0.0)).abs() < 1e-5, "and so is three o'clock");
    assert!(q(polar(c, 3.0, TAU * 0.6)) > 0.0, "past it is outside");
    assert!(q((0.0, -1.0)) > 0.0, "the hole is outside the band");
    assert!(q((0.0, -5.0)) > 0.0, "as is past the rim");
    // Whole circle: every angle is in, and the hole is still a hole.
    let all = |p| sector(p, c, 4.0, 2.0, 0.0, TAU);
    assert!(all((-3.0, 0.0)) < 0.0 && all((0.0, 1.0)) > 0.0);
    // No hole asked for: the centre is the deepest point in, not an edge.
    assert!((sector((0.0, 0.0), c, 4.0, 0.0, 0.0, TAU) + 4.0).abs() < 1e-5);
}

#[test]
fn a_round_box_is_the_box_with_its_corners_cut() {
    // A 10x2 pill at the origin: fully round ends, straight sides.
    let pill = |p| round_box(p, 0.0, 0.0, 10.0, 2.0, 1.0);
    assert!(
        (pill((5.0, 1.0)) + 1.0).abs() < 1e-5,
        "the middle is r deep"
    );
    assert!(pill((5.0, 0.0)).abs() < 1e-5, "the flat side is the edge");
    assert!(pill((0.0, 0.0)) > 0.0, "the square corner is cut away");
    assert!((pill((0.0, 1.0))).abs() < 1e-5, "the cap's tip is the edge");
    // Outside, it measures the real distance, corners included.
    assert!((pill((13.0, 1.0)) - 3.0).abs() < 1e-5);
    // Radius zero is a plain rectangle.
    let sq = |p| round_box(p, 0.0, 0.0, 10.0, 2.0, 0.0);
    assert!(sq((0.0, 0.0)).abs() < 1e-5, "its corner is on the edge");
}

#[test]
fn a_capsule_measures_from_the_segment() {
    let d = capsule((5.0, 1.0), (0.0, 0.0), (10.0, 0.0), 0.25);
    assert!((d - 0.75).abs() < 1e-5, "{d}");
    // Past the end it measures from the end point, not the infinite line.
    let d = capsule((13.0, 0.0), (0.0, 0.0), (10.0, 0.0), 0.25);
    assert!((d - 2.75).abs() < 1e-5, "{d}");
}

#[test]
fn a_cone_is_thick_at_its_base_and_a_point_at_its_tip() {
    let (a, b) = ((0.0, 0.0), (0.0, -4.0));
    let n = |p| cone(p, a, b, 0.5, 0.02);
    assert!(n((0.4, 0.0)) < 0.0, "wide at the hub");
    assert!(n((0.4, -4.0)) > 0.0, "and narrow at the tip");
    assert!(n((0.0, -4.0)) < 0.0, "the tip itself is still drawn");
}

#[test]
fn bounds_wraps_the_points_and_the_pad() {
    let b = bounds(&[(2.0, 5.0), (6.0, 1.0)], 0.5);
    assert_eq!(b, (1.5, 0.5, 5.0, 5.0));
    assert_eq!(
        bounds(&[], 1.0),
        (0.0, 0.0, 0.0, 0.0),
        "nothing bounds nothing"
    );
}

#[test]
fn polar_puts_noon_up_and_three_oclock_right() {
    let (x, y) = polar((10.0, 10.0), 4.0, 0.0);
    assert!((x - 10.0).abs() < 1e-5 && (y - 6.0).abs() < 1e-5, "{x},{y}");
    let (x, y) = polar((10.0, 10.0), 4.0, TAU * 0.25);
    assert!(
        (x - 14.0).abs() < 1e-5 && (y - 10.0).abs() < 1e-5,
        "{x},{y}"
    );
}

//! Shapes as *distance*, not as membership.
//!
//! [`Canvas::fill`](crate::plot::Canvas::fill) takes an inside/outside
//! predicate and samples it on a 3×3 grid, which buys ten coverage levels and
//! nothing finer. At the size the nav draws a gauge — a ring twenty device
//! pixels across, rasterized at four canvas pixels per cell — ten levels on a
//! half-resolution grid is exactly the staircase you can see on the arc: the
//! edge steps in blocks two device pixels wide, and any feature thinner than a
//! canvas pixel (a tick, a needle) can fall between all nine samples and draw
//! *nothing*.
//!
//! A signed distance says how far a point is from the shape's edge — negative
//! inside, positive outside, in canvas units. Coverage then comes out of the
//! distance analytically (`0.5 - d/pixel`), so an edge lands anywhere on a
//! continuous ramp and a hairline thinner than a pixel comes out grey instead
//! of absent. Every function here returns units, so they compose: `min` is
//! union, `max` is intersection, and subtracting a constant grows the shape by
//! that much in every direction.
//!
//! Angles follow the rest of `plot`: clockwise from twelve o'clock, in
//! radians, with `y` growing downward on the screen.
use std::f32::consts::{PI, TAU};

/// Distance to a disc of radius `r` centred at `c`.
pub fn disc(p: (f32, f32), c: (f32, f32), r: f32) -> f32 {
    (p.0 - c.0).hypot(p.1 - c.1) - r
}

/// Distance to a ring of radius `r` and half-thickness `w` — the full circle.
pub fn ring(p: (f32, f32), c: (f32, f32), r: f32, w: f32) -> f32 {
    ((p.0 - c.0).hypot(p.1 - c.1) - r).abs() - w
}

/// Distance to a round-capped arc: the band of half-thickness `w` at radius
/// `r`, from angle `from` clockwise to `to`.
///
/// The caps are the shape, not a pair of dots drawn over it. The old gauge
/// stamped a circle at each end to keep a 2% reading visible, which put a
/// full-thickness blob at the head of every sweep and two overlapping blobs on
/// a full one; here a sweep of nothing *is* a dot, and one of everything is a
/// seamless ring, out of the same expression.
pub fn arc(p: (f32, f32), c: (f32, f32), r: f32, w: f32, from: f32, to: f32) -> f32 {
    let sweep = (to - from).max(0.0);
    if sweep >= TAU - 1e-4 {
        return ring(p, c, r, w);
    }
    // Into "maths up" coordinates, then rotated so the arc straddles noon:
    // the distance below only has to know the half-aperture.
    let (u, v) = (p.0 - c.0, -(p.1 - c.1));
    let (sm, cm) = ((from + to) * 0.5).sin_cos();
    let (px, py) = ((u * cm - v * sm).abs(), v * cm + u * sm);
    let (sa, ca) = (sweep * 0.5).clamp(0.0, PI).sin_cos();
    if ca * px > sa * py {
        // Past the end of the sweep: the nearest point on the shape is the
        // cap's centre.
        (px - sa * r).hypot(py - ca * r) - w
    } else {
        (px.hypot(py) - r).abs() - w
    }
}

/// Distance to a rounded rectangle: the box from `(x, y)` spanning `(w, h)`,
/// with corners of radius `r`. `r` at half the shorter side is a capsule —
/// the pill every meter, thumb and progress bar in the app is.
///
/// The corner radius on those is often *smaller than a canvas pixel*, which
/// is exactly the case a sampled predicate cannot express: the corner either
/// snaps square or vanishes, and a pill drawn on a four-pixel grid comes back
/// a blunt rectangle. As a distance it rounds by the fraction of a pixel it
/// asked for.
pub fn round_box(p: (f32, f32), x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let r = r.min(w * 0.5).min(h * 0.5).max(0.0);
    // Distance from the centre, folded into one quadrant, measured against
    // the box shrunk by the corner radius.
    let (qx, qy) = (
        (p.0 - (x + w * 0.5)).abs() - (w * 0.5 - r),
        (p.1 - (y + h * 0.5)).abs() - (h * 0.5 - r),
    );
    let outside = qx.max(0.0).hypot(qy.max(0.0));
    outside + qx.max(qy).min(0.0) - r
}

/// Distance to an annulus sector: the band between `r_in` and `r_out`, from
/// angle `from` clockwise to `to`. `r_in` of zero gives a pie slice.
///
/// Square ends, unlike [`arc`] — a slice of a pie is cut on a radius, not
/// rounded off. Composed as the annulus intersected with the angular wedge:
/// `max` of two distances, which slightly overstates the distance just
/// outside the two corners and nowhere else.
pub fn sector(p: (f32, f32), c: (f32, f32), r_out: f32, r_in: f32, from: f32, to: f32) -> f32 {
    let sweep = (to - from).max(0.0);
    let (u, v) = (p.0 - c.0, -(p.1 - c.1));
    let full = sweep >= TAU - 1e-4;
    let (sm, cm) = ((from + to) * 0.5).sin_cos();
    let (px, py) = ((u * cm - v * sm).abs(), v * cm + u * sm);
    let rad = px.hypot(py);
    // A hole of nothing is a disc, and the band expression would call the
    // exact centre an edge rather than the deepest point inside.
    let band = if r_in <= 0.0 {
        rad - r_out
    } else {
        (rad - (r_out + r_in) * 0.5).abs() - (r_out - r_in) * 0.5
    };
    if full {
        return band;
    }
    let (sa, ca) = (sweep * 0.5).clamp(0.0, PI).sin_cos();
    band.max(px * ca - py * sa)
}

/// Distance to a round-capped line from `a` to `b` of half-thickness `r`.
pub fn capsule(p: (f32, f32), a: (f32, f32), b: (f32, f32), r: f32) -> f32 {
    let (pax, pay) = (p.0 - a.0, p.1 - a.1);
    let (bax, bay) = (b.0 - a.0, b.1 - a.1);
    let len2 = bax * bax + bay * bay;
    let h = if len2 <= 0.0 {
        0.0
    } else {
        ((pax * bax + pay * bay) / len2).clamp(0.0, 1.0)
    };
    (pax - bax * h).hypot(pay - bay * h) - r
}

/// The same, tapering from half-thickness `ra` at `a` to `rb` at `b` — a
/// needle: wide at the hub, a point at the tip.
///
/// This is the capsule's projection with the radius interpolated along it,
/// which is a true distance only where the taper is gentle; at a needle's
/// proportions the error is a fraction of a canvas pixel, and it shows up as
/// an edge a shade softer than it should be rather than as a wrong shape.
pub fn cone(p: (f32, f32), a: (f32, f32), b: (f32, f32), ra: f32, rb: f32) -> f32 {
    let (pax, pay) = (p.0 - a.0, p.1 - a.1);
    let (bax, bay) = (b.0 - a.0, b.1 - a.1);
    let len2 = bax * bax + bay * bay;
    let h = if len2 <= 0.0 {
        0.0
    } else {
        ((pax * bax + pay * bay) / len2).clamp(0.0, 1.0)
    };
    (pax - bax * h).hypot(pay - bay * h) - (ra + (rb - ra) * h)
}

/// A box that contains every point in `pts`, grown by `pad` on all sides:
/// what to hand a fill so it visits the pixels the shape can reach and no
/// others.
///
/// A distance field is defined everywhere, so a fill will happily evaluate it
/// over a whole dial's face to draw one tick. Eleven ticks doing that is most
/// of the cost of drawing a dial, and all of it is answering "no".
pub fn bounds(pts: &[(f32, f32)], pad: f32) -> (f32, f32, f32, f32) {
    let mut lo = (f32::MAX, f32::MAX);
    let mut hi = (f32::MIN, f32::MIN);
    for p in pts {
        lo = (lo.0.min(p.0), lo.1.min(p.1));
        hi = (hi.0.max(p.0), hi.1.max(p.1));
    }
    if lo.0 > hi.0 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    (
        lo.0 - pad,
        lo.1 - pad,
        hi.0 - lo.0 + 2.0 * pad,
        hi.1 - lo.1 + 2.0 * pad,
    )
}

/// The point at `angle` and radius `r` around `centre` — where a tick starts,
/// where a needle points, where a label goes.
pub fn polar(centre: (f32, f32), r: f32, angle: f32) -> (f32, f32) {
    (centre.0 + r * angle.sin(), centre.1 - r * angle.cos())
}

#[cfg(test)]
mod tests {
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
}

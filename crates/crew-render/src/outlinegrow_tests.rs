use swash::zeno::{Point, Verb};

use super::grow;

/// A rectangle as one closed contour, wound the way `clockwise` says (y up).
fn rect(x0: f32, y0: f32, x1: f32, y1: f32, clockwise: bool) -> (Vec<Point>, Vec<Verb>) {
    let mut points = vec![
        Point::new(x0, y0),
        Point::new(x1, y0),
        Point::new(x1, y1),
        Point::new(x0, y1),
    ];
    if clockwise {
        points.reverse();
    }
    let verbs = vec![
        Verb::MoveTo,
        Verb::LineTo,
        Verb::LineTo,
        Verb::LineTo,
        Verb::Close,
    ];
    (points, verbs)
}

fn width(points: &[Point]) -> f32 {
    let xs = || points.iter().map(|p| p.x);
    xs().fold(f32::MIN, f32::max) - xs().fold(f32::MAX, f32::min)
}

/// The signed area swash takes: every point of the glyph as one polygon.
fn area_as_one_polygon(points: &[Point]) -> f32 {
    let mut prev = points[points.len() - 1];
    let mut area = 0.0;
    for cur in points {
        area += (cur.y - prev.y) * (cur.x + prev.x);
        prev = *cur;
    }
    area
}

/// Reported 2026-11-11, "letter i rendering in different color": a stem with
/// a dot above it is two contours, and read as one polygon the jump between
/// them can outweigh both — the `i` was thinned by what it should have
/// gained. Every way a font may wind the two and every corner it may start
/// each from; some of those are the ones the one-polygon reading gets wrong.
#[test]
fn a_dotted_stem_grows_however_it_is_wound_or_started() {
    let mut misread = 0;
    for clockwise in [true, false] {
        for (stem_start, dot_start) in (0..4).flat_map(|s| (0..4).map(move |d| (s, d))) {
            let mut stem = rect(4.0, 0.0, 6.0, 10.0, clockwise);
            let mut dot = rect(3.5, 13.0, 6.5, 16.0, clockwise);
            stem.0.rotate_left(stem_start);
            dot.0.rotate_left(dot_start);
            let mut points = [stem.0.clone(), dot.0.clone()].concat();
            let verbs = [stem.1, dot.1].concat();
            let truth = area_as_one_polygon(&stem.0) + area_as_one_polygon(&dot.0);
            misread += usize::from(truth * area_as_one_polygon(&points) < 0.0);
            grow(&mut points, &verbs, 0.5);
            for (was, now) in [(&stem.0, &points[..4]), (&dot.0, &points[4..])] {
                let gained = width(now) - width(was);
                assert!(
                    (gained - 1.0).abs() < 1e-4,
                    "clockwise {clockwise}, starts {stem_start}/{dot_start}: gained {gained} px"
                );
            }
        }
    }
    assert!(misread > 0, "no arrangement here is one swash misreads");
}

/// A counter winds against its glyph, and growing the glyph closes it in:
/// the stroke between the two gets wider from both sides.
#[test]
fn a_counter_shrinks_as_its_glyph_grows() {
    for clockwise in [true, false] {
        let outer = rect(0.0, 0.0, 10.0, 10.0, clockwise);
        let hole = rect(3.0, 3.0, 7.0, 7.0, !clockwise);
        let mut points = [outer.0.clone(), hole.0.clone()].concat();
        let verbs = [outer.1, hole.1].concat();
        grow(&mut points, &verbs, 0.5);
        assert!((width(&points[..4]) - 11.0).abs() < 1e-4, "the outer shape");
        assert!((width(&points[4..]) - 3.0).abs() < 1e-4, "the counter");
    }
}

/// The left and bottom edges stay where the font put them — what the
/// offset in `outlineink` is worked out against.
#[test]
fn growth_goes_up_and_right() {
    let (mut points, verbs) = rect(2.0, 1.0, 4.0, 9.0, true);
    grow(&mut points, &verbs, 0.5);
    let min = |f: fn(&Point) -> f32| points.iter().map(f).fold(f32::MAX, f32::min);
    assert!((min(|p| p.x) - 2.0).abs() < 1e-4);
    assert!((min(|p| p.y) - 1.0).abs() < 1e-4);
}

use super::*;

const FILL: (u8, u8, u8) = (0, 255, 0);
const TRACK: (u8, u8, u8) = (150, 150, 150);
const DIM: (u8, u8, u8) = (70, 70, 70);
const PLATE: Option<((u8, u8, u8), f32)> = Some(((20, 20, 30), 0.5));

fn dial(frac: f32) -> Canvas {
    let mut c = Canvas::with_sub(9, 3, 2.0, 16);
    draw(
        &mut c,
        Dial {
            centre: (4.5, 3.0),
            r: 2.8,
            frac,
            color: FILL,
            track: TRACK,
            track_dim: DIM,
            plate: PLATE,
        },
    );
    c
}

/// Two colours within rounding of each other: the canvas stores
/// premultiplied and un-premultiplies at emit, and every mark here is
/// drawn over a translucent plate.
fn near(a: (u8, u8, u8), b: (u8, u8, u8)) -> bool {
    let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
    d(a.0, b.0) + d(a.1, b.1) + d(a.2, b.2) < 40
}

/// Ink of one colour inside a box, in square units.
fn ink_in(c: &Canvas, color: (u8, u8, u8), b: (f32, f32, f32, f32)) -> f32 {
    c.paint()
        .iter()
        .filter(|p| near(p.color, color))
        .map(|p| {
            let w = (p.x + p.w).min(b.0 + b.2) - p.x.max(b.0);
            let h = ((p.y + p.h).min(b.1 + b.3) - p.y.max(b.1)) * c.row_units();
            w.max(0.0) * h.max(0.0) * p.alpha
        })
        .sum()
}

fn ink(c: &Canvas, color: (u8, u8, u8)) -> f32 {
    ink_in(c, color, (0.0, 0.0, 100.0, 100.0))
}

/// The whole point of a dial: the hand moves, and where it points is the
/// reading. Empty points down-left, half points straight up, full points
/// down-right.
#[test]
fn the_needle_points_where_the_reading_is() {
    // A box on the hand's shaft, inside the tick ring — measuring out at
    // the tip would count the lit ticks the hand has swept past, and a
    // full dial lights every tick on the face.
    let hand_box = |frac: f32| {
        let (x, y) = sdf::polar((4.5, 3.0), 2.8 * 0.45, angle_of(frac));
        (x - 0.4, (y - 0.4) / 2.0, 0.8, 0.8 / 2.0)
    };
    for frac in [0.0, 0.5, 1.0] {
        let c = dial(frac);
        let here = ink_in(&c, FILL, hand_box(frac));
        assert!(here > 0.05, "the hand reaches {frac}: {here}");
        for other in [0.0, 0.5, 1.0] {
            if (other - frac).abs() < 1e-6 {
                continue;
            }
            let there = ink_in(&c, FILL, hand_box(other));
            assert!(
                there < here * 0.3,
                "and not to {other} as well ({there} vs {here})"
            );
        }
    }
}

/// The scale is fixed, so an empty dial is still a dial: the ticks and
/// the bezel are there to read the hand against.
#[test]
fn an_empty_dial_still_shows_the_scale_it_would_fill() {
    let c = dial(0.0);
    let scale = ink(&c, TRACK) + ink(&c, DIM);
    assert!(scale > 1.0, "bezel and ticks: {scale}");
}

/// …and the ticks the hand has passed carry the reading too, so it is
/// said by length as well as by angle.
#[test]
fn the_lit_ticks_follow_the_reading() {
    // Counted at the ticks' outer ends, past where the hand reaches: ink
    // summed over the whole face would be measuring the needle.
    let lit = |frac: f32| {
        let c = dial(frac);
        (0..TICKS)
            .filter(|i| {
                let t = *i as f32 / (TICKS - 1) as f32;
                let (x, y) = sdf::polar((4.5, 3.0), 2.8 * 0.87, angle_of(t));
                ink_in(&c, FILL, (x - 0.25, (y - 0.25) / 2.0, 0.5, 0.5 / 2.0)) > 0.01
            })
            .count()
    };
    assert_eq!(lit(0.0), 1, "empty lights only the tick at zero");
    assert_eq!(lit(0.5), 6, "half lights half of them");
    assert_eq!(lit(1.0), TICKS, "full lights the scale");
}

/// A face with room for it gets a finer scale — every twentieth rather
/// than every tenth — and the small one must not, or its ticks would run
/// into each other.
#[test]
fn a_large_face_gets_a_finer_scale() {
    let ticks_on = |r: f32| {
        let mut c = Canvas::with_sub(16, 6, 2.0, 16);
        draw(
            &mut c,
            Dial {
                centre: (8.0, 6.0),
                r,
                frac: 1.0,
                color: FILL,
                track: TRACK,
                track_dim: DIM,
                plate: PLATE,
            },
        );
        // Count the lit ticks out past where the hand reaches.
        (0..TICKS_LARGE)
            .filter(|i| {
                let t = *i as f32 / (TICKS_LARGE - 1) as f32;
                let (x, y) = sdf::polar((8.0, 6.0), r * 0.87, angle_of(t));
                ink_in(&c, FILL, (x - 0.2, (y - 0.2) / 2.0, 0.4, 0.4 / 2.0)) > 0.01
            })
            .count()
    };
    assert_eq!(ticks_on(LARGE_R + 1.0), TICKS_LARGE, "the large scale");
    // On a small face only the every-tenth marks exist, so half the
    // large scale's positions are bare.
    assert_eq!(ticks_on(LARGE_R - 1.5), TICKS, "the small one");
}

/// The bottom of the face is deliberately open — it is where the digits
/// go. Nothing on the scale, bezel included, may be drawn across it.
#[test]
fn the_scale_leaves_the_bottom_of_the_face_clear() {
    let c = dial(0.5);
    // Under the hub, from inside the tick ring out to just short of the
    // bezel: two columns wide, the space two digits ask for.
    let (r, cy) = (2.8, 3.0);
    let below = (4.5 - 1.0, (cy + 0.55 * r) / 2.0, 2.0, 0.45 * r / 2.0);
    let marks = ink_in(&c, TRACK, below) + ink_in(&c, DIM, below) + ink_in(&c, FILL, below);
    assert!(marks < 0.05, "the digits' gap is clear: {marks}");
}

/// A dial with no radius is not a crash and not a dot.
#[test]
fn a_dial_with_no_room_draws_nothing() {
    let mut c = Canvas::new(9, 3, 2.0);
    draw(
        &mut c,
        Dial {
            centre: (4.5, 3.0),
            r: 0.0,
            frac: 0.5,
            color: FILL,
            track: TRACK,
            track_dim: DIM,
            plate: PLATE,
        },
    );
    assert!(c.paint().is_empty());
}

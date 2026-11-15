use super::*;

/// 14 pt on a Retina display, at crew's 1.25 cell and at `loose`.
const SIZE: f32 = 28.0;
const NORMAL: f32 = 35.0;
const LOOSE: f32 = 45.0;

fn lilex() -> Face {
    let mut fs = crate::embedfont::font_system();
    face(&mut fs, None, 500).expect("the embedded face has a post table")
}

fn baseline(f: Face, cell_h: f32) -> f32 {
    (cell_h - (f.ascent + f.descent) * SIZE) / 2.0 + f.ascent * SIZE
}

#[test]
fn the_embedded_face_says_where_its_rules_go() {
    let f = lilex();
    assert!(
        f.under < 0.0 && f.under > -0.3,
        "underline {:.3} em",
        f.under
    );
    assert!(
        f.strike > 0.1 && f.strike < 0.4,
        "strike {:.3} em",
        f.strike
    );
}

/// At crew's own cell the underline stays where it was for a face whose
/// table agrees (Lilex asks for 31.2 px, crew drew 31).
#[test]
fn at_the_normal_cell_a_well_made_face_keeps_its_underline() {
    let r = Rules::from_face(lilex(), SIZE, NORMAL);
    assert_eq!(r.under, Rules::plain(NORMAL).under);
}

/// A short face with an empty table (both Comic faces): the underline
/// follows its baseline up instead of hanging at the cell's floor.
#[test]
fn a_face_that_does_not_say_gets_the_common_place() {
    let comic = Face {
        ascent: 0.75,
        descent: 0.25,
        under: FALLBACK_UNDER,
        strike: 0.225,
    };
    let r = Rules::from_face(comic, SIZE, NORMAL);
    let gap = r.under - baseline(comic, NORMAL);
    let old = Rules::plain(NORMAL).under - baseline(comic, NORMAL);
    assert!((1.0..=0.15 * SIZE).contains(&gap), "{gap:.1} px under");
    assert!(old - gap >= 3.0, "was {old:.1} px off, now {gap:.1}");
}

/// Air between lines is air: the underline stays with its word.
#[test]
fn leading_does_not_pull_the_underline_off_its_word() {
    let f = lilex();
    let gap = |h: f32| Rules::from_face(f, SIZE, h).under - baseline(f, h);
    assert!(
        (gap(LOOSE) - gap(NORMAL)).abs() <= 1.0,
        "{} vs {}",
        gap(LOOSE),
        gap(NORMAL)
    );
    let old = Rules::plain(LOOSE).under - baseline(f, LOOSE);
    assert!(
        old - gap(LOOSE) >= 2.5,
        "was {old:.1} px off, now {:.1}",
        gap(LOOSE)
    );
}

/// The strike crosses the middle of the lower case, not its top.
#[test]
fn the_strike_crosses_the_x_height() {
    let mut fs = crate::embedfont::font_system();
    let font = crate::sizeadjust::font(&mut fs, None, 500).unwrap();
    let m = font.metrics();
    let xh = m.x_height.unwrap() / f32::from(m.units_per_em) * SIZE;
    let f = lilex();
    let r = Rules::from_face(f, SIZE, NORMAL);
    let mid = r.strike + crate::deco::thickness(NORMAL) / 2.0;
    let (top, foot) = (baseline(f, NORMAL) - xh, baseline(f, NORMAL));
    assert!(
        mid > top + 0.3 * xh && mid < foot - 0.3 * xh,
        "{mid} not in {top}..{foot}"
    );
    assert!(r.strike >= Rules::plain(NORMAL).strike + 2.0, "{r:?}");
}

/// A face whose numbers make no sense, a squat cell: everything stays inside.
#[test]
fn the_rules_stay_in_their_cell() {
    let odd = Face {
        ascent: 1.2,
        descent: 0.5,
        under: -0.9,
        strike: 2.0,
    };
    for h in [12.0, 20.0, NORMAL, LOOSE] {
        let r = Rules::from_face(odd, SIZE * h / NORMAL, h);
        let t = crate::deco::thickness(h);
        assert!(r.under + 2.0 * t <= h, "{h}: {r:?}");
        assert!(r.strike >= 0.0 && r.strike + t <= h, "{h}: {r:?}");
    }
}

/// A double underline is two lines inside the cell; where the cell has room
/// under a single, the upper one is where a single would be.
#[test]
fn a_double_rule_goes_down_from_the_single_where_there_is_room() {
    use crew_theme::deco::{Deco, DecoLine};
    for h in [NORMAL, LOOSE] {
        let r = Rules::from_face(lilex(), SIZE, h);
        let rule = |l| crate::deco::rects_at(&Deco::underline(l), 0.0, 0.0, 17.0, h, r);
        let (one, two) = (rule(DecoLine::Single), rule(DecoLine::Double));
        assert!(two.iter().all(|q| q.1 + q.3 <= h), "{h}: {two:?}");
        if h == LOOSE {
            assert_eq!(two[0].1, one[0].1);
        }
    }
}

/// What a frame asks for: the face its params name, at their size, read
/// once and kept while nothing changes.
#[test]
fn a_frame_gets_its_faces_rules() {
    let mut fs = crate::embedfont::font_system();
    let p = FontParams {
        font_size: SIZE,
        line_height: LOOSE,
        cell_w: (SIZE * 0.6).round(),
        family: None,
        weight: 500,
        smooth: 0,
        gamma: 0,
        dark: true,
        body: ((255, 255, 255), (0, 0, 0)),
    };
    let r = for_params(&mut fs, &p, LOOSE);
    assert_eq!(r, Rules::from_face(lilex(), SIZE, LOOSE));
    assert_ne!(r, Rules::plain(LOOSE));
    assert_eq!(
        for_params(&mut fs, &p, LOOSE),
        r,
        "the memo answers the same"
    );
}

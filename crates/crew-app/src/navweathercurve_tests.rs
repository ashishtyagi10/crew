use super::*;

fn berlin() -> Weather {
    Weather {
        place: "Berlin".into(),
        temp: 24,
        hi: 27,
        lo: 18,
        rain: 10,
        code: 0,
        unit: 'C',
        hours: Vec::new(),
        hour: 0,
    }
}

/// A day of hours draws two rows of curve under the words, in the card's
/// rows and columns; a reading without hours draws nothing and takes the
/// shorter block. The curve is scaled over the day, never past it.
#[test]
fn the_hours_draw_a_curve_under_the_words() {
    let _g = crate::palette::test_guard();
    let _t = crate::app::theme_test_guard();
    let flat = curve(&[20, 20, 20]);
    assert!(flat.iter().all(|v| (v - 0.5).abs() < 1e-6), "{flat:?}");
    let day = curve(&[18, 22, 27, 21]);
    assert_eq!((day[0], day[2]), (0.0, 1.0), "{day:?}");
    let bare = berlin();
    assert!(weather_paint(&bare, 10, 34, 2.0).is_empty());
    let w = Weather {
        hours: (0..24).map(|h| 18 + (h * 7 / 12).min(9)).collect(),
        ..berlin()
    };
    let paint = weather_paint(&w, 10, 34, 2.0);
    assert!(paint.len() > 20, "{}", paint.len());
    let (lo, hi) = paint.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
        (lo.min(p.y), hi.max(p.y + p.h))
    });
    assert!(lo >= 13.0 - 1e-3 && hi <= 15.0 + 1e-3, "rows {lo}..{hi}");
    assert!(paint.iter().all(|p| p.x >= 2.0 && p.x + p.w <= 33.0));
    // The dot is on now — the left end — and nothing solid stands on the
    // right end, where a live chart's head would.
    let solid = |p: &&Paint| p.alpha >= 0.99 && p.color == crate::palette::accent();
    assert!(
        paint.iter().filter(solid).any(|p| p.x < 3.0),
        "a dot at now"
    );
    assert!(
        !paint
            .iter()
            .filter(solid)
            .any(|p| p.x + p.w > 32.4 && p.h > 0.3),
        "no head on tomorrow"
    );
}

/// Midnight is `24 - hour` hours in, as a fraction of the curve; a curve
/// that starts at midnight gets no tick, nor one too short to reach it.
#[test]
fn midnight_falls_where_the_hours_say() {
    assert_eq!(midnight_at(8, 24), Some(16.0 / 24.0));
    assert_eq!(midnight_at(23, 24), Some(1.0 / 24.0));
    assert_eq!(midnight_at(0, 24), None);
    assert_eq!(midnight_at(8, 12), None, "the curve ends before it");
}

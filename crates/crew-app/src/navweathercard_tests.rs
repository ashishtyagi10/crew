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

fn line(cells: &[CellView], r: u16) -> String {
    let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == r).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect::<String>().trim().to_string()
}

/// The rule names the place, the first row says the sky in words with the
/// reading in the accent, the second says today's range and the rain —
/// and the block is exactly the rows the layout reserves for it.
#[test]
fn the_card_says_the_place_the_sky_and_the_range() {
    let _g = crate::palette::test_guard();
    let _t = crate::app::theme_test_guard();
    let cells = weather_cells(&berlin(), 34);
    assert!(
        line(&cells, 0).contains("WEATHER Berlin"),
        "{}",
        line(&cells, 0)
    );
    assert_eq!(line(&cells, 1), "\u{2600} 24\u{00b0}C clear");
    assert_eq!(line(&cells, 2), "\u{2191}27 \u{2193}18  \u{2602}10%");
    assert!(
        cells.iter().all(|c| c.row < WEATHER_BLOCK - 1),
        "rows + gap"
    );
    let acc = crate::palette::accent();
    assert!(cells
        .iter()
        .any(|c| c.row == 1 && c.c == '2' && c.fg == acc));
    assert!(cells.iter().any(|c| c.row == 1 && c.c == '2' && c.bold));
    assert!(cells
        .iter()
        .any(|c| c.row == 1 && c.c == 'l' && c.fg != acc && !c.bold));
    let dry = weather_cells(
        &Weather {
            rain: 0,
            ..berlin()
        },
        34,
    );
    assert_eq!(
        line(&dry, 2),
        "\u{2191}27 \u{2193}18",
        "no rain, no umbrella"
    );
}

/// On a narrow nav the words go before the reading does, then the unit;
/// the place leaves the rule before the title would.
#[test]
fn a_narrow_nav_keeps_the_reading_and_drops_the_words() {
    let _g = crate::palette::test_guard();
    let _t = crate::app::theme_test_guard();
    let w = Weather {
        code: 2,
        ..berlin()
    };
    let wide = weather_cells(&w, 40);
    assert_eq!(line(&wide, 1), "\u{2601} 24\u{00b0}C partly cloudy");
    let mid = weather_cells(&w, 22);
    assert_eq!(line(&mid, 1), "\u{2601} 24\u{00b0} partly cloudy");
    let narrow = weather_cells(&w, 14);
    assert_eq!(line(&narrow, 1), "\u{2601} 24\u{00b0}C");
    assert!(line(&narrow, 0).contains("WEATHER"));
    assert!(!line(&narrow, 0).contains("Berlin"));
    assert!(!line(&narrow, 1).contains('\u{2026}'));
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
    assert_eq!(block(&bare), WEATHER_BLOCK);
    assert!(weather_paint(&bare, 10, 34, 2.0).is_empty());
    let w = Weather {
        hours: (0..24).map(|h| 18 + (h * 7 / 12).min(9)).collect(),
        ..berlin()
    };
    assert_eq!(block(&w), WEATHER_BLOCK + CHART_ROWS);
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

#[test]
fn every_wmo_family_has_words() {
    for code in [0, 1, 2, 3, 45, 51, 56, 61, 66, 71, 77, 80, 85, 95, 96] {
        assert!(!condition(code).is_empty());
    }
    assert_eq!(condition(0), "clear");
    assert_eq!(condition(63), "rain");
    assert_eq!(condition(99), "thunder, hail");
}

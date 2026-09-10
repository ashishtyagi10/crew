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

/// A lookup in flight and a place not found each draw one muted row under
/// the rule — the place on it while looking, the fix in the row when not
/// found — and take the quiet block; off draws nothing and takes none.
#[test]
fn the_quiet_states_say_what_the_card_waits_on() {
    let _g = crate::palette::test_guard();
    let _t = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    let looking = state_cells(&State::Looking("New York".into()), 34);
    assert!(
        line(&looking, 0).contains("WEATHER New York"),
        "{}",
        line(&looking, 0)
    );
    assert_eq!(line(&looking, 1), "looking up\u{2026}");
    assert!(looking.iter().all(|c| c.row < QUIET_BLOCK - 1));
    assert!(looking
        .iter()
        .filter(|c| c.row == 1)
        .all(|c| c.fg == t.text_muted));
    let missing = state_cells(&State::Missing("Nowhere".into()), 60);
    assert!(
        !line(&missing, 0).contains("Nowhere"),
        "the rule names no place"
    );
    assert_eq!(
        line(&missing, 1),
        "nothing for Nowhere \u{2014} /weather <place>"
    );
    let narrow = state_cells(&State::Missing("Nowhere".into()), 26);
    assert_eq!(
        line(&narrow, 1),
        "nothing for Nowhere",
        "the fix goes whole"
    );
    assert_eq!(block(&State::Looking("x".into())), QUIET_BLOCK);
    assert_eq!(block(&State::Missing("x".into())), QUIET_BLOCK);
    assert_eq!(block(&State::Off), 0);
    assert!(state_cells(&State::Off, 34).is_empty());
    let found = state_cells(&State::Found(berlin()), 34);
    assert_eq!(line(&found, 1), line(&weather_cells(&berlin(), 34), 1));
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

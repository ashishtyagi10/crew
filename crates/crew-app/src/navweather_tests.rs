use super::*;

const GEO: &str = r#"{"results":[{"name":"Berlin","latitude":52.52437,"longitude":13.41053,"country_code":"DE"}]}"#;
const FORECAST: &str = r#"{"current":{"time":"2026-09-10T08:15","temperature_2m":23.6,"weather_code":61},"hourly":{"time":["2026-09-10T06:00","2026-09-10T07:00","2026-09-10T08:00","2026-09-10T09:00","2026-09-10T10:00"],"temperature_2m":[17.9,19.2,21.4,23.1,24.9]},"daily":{"temperature_2m_max":[27.1],"temperature_2m_min":[17.8],"precipitation_probability_max":[35]}}"#;

/// Real Open-Meteo shapes (2026-09) parse into one reading, rounded, with
/// the place the geocoder names and the unit the country decides.
#[test]
fn geocoder_and_forecast_bodies_parse_into_a_reading() {
    let (lat, lon, name, cc) = parse_geo(GEO).unwrap();
    assert!((lat - 52.524).abs() < 0.01 && (lon - 13.41).abs() < 0.01);
    assert_eq!((name.as_str(), cc.as_str()), ("Berlin", "DE"));
    let w = parse_forecast(FORECAST, &name, 'C').unwrap();
    assert_eq!(
        w,
        Weather {
            place: "Berlin".into(),
            temp: 24,
            hi: 27,
            lo: 18,
            rain: 35,
            code: 61,
            unit: 'C',
            hours: vec![21, 23, 25],
            hour: 8,
        }
    );
    let bare = FORECAST.replace(r#""time":"2026-09-10T08:15","#, "");
    assert_eq!(
        parse_forecast(&bare, "x", 'C').unwrap().hours,
        Vec::<i32>::new(),
        "no current hour, no series"
    );
    assert_eq!(
        line(&w),
        "\u{2602} 24\u{00b0} \u{2191}27 \u{2193}18 \u{2602}35%"
    );
    assert_eq!(
        parse_geo(r#"{"results":[]}"#),
        None,
        "an unknown place is None"
    );
    assert_eq!(parse_geo("nope"), None);
    assert_eq!(parse_forecast("{}", "x", 'C'), None);
}

/// No rain chance, no rain segment; the glyph follows the WMO families.
#[test]
fn a_dry_clear_day_reads_short_and_glyphs_follow_the_code() {
    let w = Weather {
        place: "x".into(),
        temp: 31,
        hi: 33,
        lo: 22,
        rain: 0,
        code: 0,
        unit: 'F',
        hours: Vec::new(),
        hour: 0,
    };
    assert_eq!(line(&w), "\u{2600} 31\u{00b0} \u{2191}33 \u{2193}22");
    assert_eq!(glyph(2), '\u{2601}');
    assert_eq!(glyph(45), '\u{2248}');
    assert_eq!(glyph(80), '\u{2602}');
    assert_eq!(glyph(75), '\u{2744}');
    assert_eq!(glyph(95), '\u{26a1}');
}

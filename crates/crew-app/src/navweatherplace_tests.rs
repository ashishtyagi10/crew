use super::*;

/// The zone's last segment is the city, spaces restored; zones that name
/// an offset or nothing at all give no place rather than a nonsense query.
#[test]
fn a_zone_names_its_city_or_nothing() {
    assert_eq!(city_of("America/New_York").as_deref(), Some("New York"));
    assert_eq!(
        city_of("America/Argentina/Buenos_Aires").as_deref(),
        Some("Buenos Aires")
    );
    assert_eq!(city_of("Asia/Kolkata").as_deref(), Some("Kolkata"));
    assert_eq!(
        city_of("America/Port_of_Spain").as_deref(),
        Some("Port of Spain")
    );
    for none in [
        "UTC",
        "Etc/UTC",
        "Etc/GMT+3",
        "EST5EDT",
        "Factory",
        "Europe/",
        "",
    ] {
        assert_eq!(city_of(none), None, "{none}");
    }
}

/// `off` is off whatever the zone; empty follows the zone; text is text.
#[test]
fn the_key_outranks_the_zone_except_when_empty() {
    let zone = Some("Europe/Berlin");
    assert_eq!(resolve_with("", zone).as_deref(), Some("Berlin"));
    assert_eq!(resolve_with("  ", zone).as_deref(), Some("Berlin"));
    assert_eq!(resolve_with("", Some("UTC")), None);
    assert_eq!(resolve_with("", None), None);
    assert_eq!(resolve_with("off", zone), None);
    assert_eq!(resolve_with(" OFF ", zone), None);
    assert_eq!(resolve_with("Tokyo", zone).as_deref(), Some("Tokyo"));
    assert!(is_auto("") && !is_auto("off") && !is_auto("Tokyo"));
}

const CHARLOTTE: Option<i32> = Some(-4 * 3600);
const HONDURAS: i32 = -6 * 3600;

/// `/weather charlotte` typed at home, the machine now in Honduras: the
/// card would show Charlotte's sky under a Tegucigalpa clock, so it says
/// which clock and how to follow it. The same offset says nothing, and a
/// place left to the clock (or switched off) never can disagree with it.
#[test]
fn a_typed_place_on_another_clock_says_which_clock() {
    let tz = Some("America/Tegucigalpa");
    let note = travel_note("charlotte", "Charlotte", CHARLOTTE, HONDURAS, tz).unwrap();
    assert_eq!(
        note,
        "weather: your clock is on Tegucigalpa time, not Charlotte\u{2019}s \
         \u{2014} /weather auto follows the clock"
    );
    let ny = Some("America/New_York");
    assert_eq!(
        travel_note("charlotte", "Charlotte", CHARLOTTE, -4 * 3600, ny),
        None
    );
    assert_eq!(travel_note("", "New York", CHARLOTTE, HONDURAS, tz), None);
    assert_eq!(
        travel_note("off", "Charlotte", CHARLOTTE, HONDURAS, tz),
        None
    );
    assert_eq!(
        travel_note("charlotte", "Charlotte", None, HONDURAS, tz),
        None
    );
    // A zone with no city names the clock by its offset.
    let utc = travel_note("charlotte", "Charlotte", CHARLOTTE, HONDURAS, None).unwrap();
    assert!(utc.contains("on UTC\u{2212}6 time"), "{utc}");
}

#[test]
fn an_offset_names_its_clock() {
    assert_eq!(utc_label(0), "UTC");
    assert_eq!(utc_label(-6 * 3600), "UTC\u{2212}6");
    assert_eq!(utc_label(5 * 3600 + 1800), "UTC+5:30");
    assert_eq!(utc_label(-(3 * 3600 + 1800)), "UTC\u{2212}3:30");
}

/// The whole trip, as `travel_check` runs it: typing the place at home
/// squares it with that clock; landing in Honduras says so on the first
/// reading and never on the next; home again is quiet but recorded, so
/// the next trip says it again.
#[test]
fn the_note_comes_once_per_arrival() {
    let dir = std::env::temp_dir().join(format!("crew-travel-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("weather.noted");
    let _ = std::fs::remove_file(&path);
    let mut said = Vec::new();
    let mut reading = |zone: &str, clock: i32| {
        let note = travel_note("charlotte", "Charlotte", CHARLOTTE, clock, Some(zone));
        if let (true, Some(n)) = (arrive_at(&path, zone), note) {
            said.push(n);
        }
    };
    assert!(arrive_at(&path, "America/New_York"), "typed at home");
    reading("America/New_York", -4 * 3600);
    reading("America/Tegucigalpa", HONDURAS);
    reading("America/Tegucigalpa", HONDURAS);
    reading("America/New_York", -4 * 3600);
    reading("America/Tegucigalpa", HONDURAS);
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said.iter().all(|n| n.contains("Tegucigalpa time")));
    // Nowhere to write means nothing recorded — not a note every reading.
    assert!(!arrive_at(
        &dir.join("gone").join("weather.noted"),
        "Europe/Berlin"
    ));
    let _ = std::fs::remove_dir_all(&dir);
}

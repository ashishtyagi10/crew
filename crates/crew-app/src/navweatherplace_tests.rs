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

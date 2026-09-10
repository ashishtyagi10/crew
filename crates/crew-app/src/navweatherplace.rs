//! Where the weather is for. The card used to exist only after someone
//! typed `/weather <place>`, which on a fresh install is a card nobody
//! has seen. The machine's time zone names a city — `America/New_York`,
//! `Asia/Kolkata` — and Open-Meteo's geocoder resolves those names, so an
//! empty key means *that* city; `/weather off` writes the word `off`, and
//! `/weather auto` empties the key again.

/// The key's value that means no card and no strip.
pub(crate) const OFF: &str = "off";

/// The city an IANA zone name carries: `America/Argentina/Buenos_Aires`
/// → `Buenos Aires`. Zones that name no city (`UTC`, `Etc/GMT+3`,
/// `EST5EDT`, `Factory`) yield nothing.
pub(crate) fn city_of(zone: &str) -> Option<String> {
    let (region, city) = zone.trim().rsplit_once('/')?;
    if region.starts_with("Etc") || city.is_empty() {
        return None;
    }
    if city.chars().any(|c| c.is_ascii_digit()) || city.contains('+') || city.contains('-') {
        return None;
    }
    Some(city.replace('_', " "))
}

/// The place to report for, from the config key and the zone: `off` is
/// none; empty is the zone's city (none when the zone has no city); any
/// other text is the place as typed. Pure.
pub(crate) fn resolve_with(key: &str, zone: Option<&str>) -> Option<String> {
    let key = key.trim();
    if key.eq_ignore_ascii_case(OFF) {
        None
    } else if key.is_empty() {
        zone.and_then(city_of)
    } else {
        Some(key.to_string())
    }
}

/// [`resolve_with`] for this machine's zone.
pub(crate) fn resolve(key: &str) -> Option<String> {
    let zone = iana_time_zone::get_timezone().ok();
    resolve_with(key, zone.as_deref())
}

/// Whether the key leaves the place to the zone (for the status line).
pub(crate) fn is_auto(key: &str) -> bool {
    key.trim().is_empty()
}

#[cfg(test)]
#[path = "navweatherplace_tests.rs"]
mod tests;

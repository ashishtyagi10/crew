//! Where the weather is for. The card used to exist only after someone
//! typed `/weather <place>`, which on a fresh install is a card nobody
//! has seen. The machine's time zone names a city — `America/New_York`,
//! `Asia/Kolkata` — and Open-Meteo's geocoder resolves those names, so an
//! empty key means *that* city; `/weather off` writes the word `off`, and
//! `/weather auto` empties the key again. A typed place stays put when
//! the machine travels, so arriving on another clock says so once.

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

/// What a typed place says once the clock has moved out from under it:
/// `charlotte` typed at home, the machine now on Tegucigalpa's clock.
/// `place_offset` is the place's UTC offset (its forecast's), `clock` the
/// machine's, and the zone names the clock's city. Auto and off never
/// carry a note — auto IS the clock's zone. Pure.
pub(crate) fn travel_note(
    key: &str,
    place: &str,
    place_offset: Option<i32>,
    clock: i32,
    zone: Option<&str>,
) -> Option<String> {
    let key = key.trim();
    if is_auto(key) || key.eq_ignore_ascii_case(OFF) || place_offset? == clock {
        return None;
    }
    let here = zone.and_then(city_of).unwrap_or_else(|| utc_label(clock));
    Some(format!(
        "weather: your clock is on {here} time, not {place}\u{2019}s \u{2014} /weather auto follows the clock"
    ))
}

/// `UTC\u{2212}6`, `UTC+5:30`, `UTC` — an offset in seconds as a clock's name.
pub(crate) fn utc_label(secs: i32) -> String {
    let (sign, a) = if secs < 0 {
        ('\u{2212}', -secs)
    } else {
        ('+', secs)
    };
    match (a / 3600, a % 3600 / 60) {
        (0, 0) => "UTC".into(),
        (h, 0) => format!("UTC{sign}{h}"),
        (h, m) => format!("UTC{sign}{h}:{m:02}"),
    }
}

/// This machine's clock: its UTC offset now, and the name it goes by —
/// the IANA zone, or the offset when the zone cannot be read.
pub(crate) fn clock() -> (i32, Option<String>, String) {
    let offset = chrono::Local::now().offset().local_minus_utc();
    let zone = iana_time_zone::get_timezone().ok();
    let name = zone.clone().unwrap_or_else(|| utc_label(offset));
    (offset, zone, name)
}

/// Record `zone` as the clock the place was last squared with; whether it
/// is a different one. A note is said once per zone the machine ARRIVES
/// in — never once per reading — and typing a place squares it with the
/// clock it was typed on, so a far city chosen on purpose stays quiet.
pub(crate) fn arrive_at(path: &std::path::Path, zone: &str) -> bool {
    if std::fs::read_to_string(path).is_ok_and(|z| z == zone) {
        return false;
    }
    std::fs::write(path, zone).is_ok()
}

/// [`arrive_at`] beside the weather cache.
pub(crate) fn arrive(zone: &str) -> bool {
    crate::navweatherfetch::cache_path()
        .is_some_and(|p| arrive_at(&p.with_extension("noted"), zone))
}

#[cfg(test)]
#[path = "navweatherplace_tests.rs"]
mod tests;

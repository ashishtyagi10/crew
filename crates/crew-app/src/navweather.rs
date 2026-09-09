//! The clock's weather strip: one line — `☀ 24° ↑27 ↓18 ☂10%` — for a place
//! the user names (`/weather Berlin`). Open-Meteo answers without a key:
//! its geocoder turns the name into a point, its forecast gives the current
//! reading, today's range and the chance of rain. Fetched on a worker
//! thread (`modelfetch`'s pattern), cached beside the config for an hour,
//! and read by the winit thread from a process-global — never a request
//! on the frame path. Crew has no location of its own (see `daylight`), so
//! the place is typed, not looked up.
use std::sync::RwLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// How long a reading is trusted before the next fetch.
pub(crate) const TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub(crate) struct Weather {
    /// The place as the geocoder names it (what the user typed, resolved).
    pub place: String,
    pub temp: i32,
    pub hi: i32,
    pub lo: i32,
    /// Today's highest chance of precipitation, in percent.
    pub rain: u8,
    /// WMO weather code, for the glyph.
    pub code: u16,
    /// `'C'` or `'F'` — Fahrenheit for a place in the US, Celsius elsewhere.
    pub unit: char,
}

static CURRENT: RwLock<Option<Weather>> = RwLock::new(None);

pub(crate) fn set(w: Option<Weather>) {
    if let Ok(mut g) = CURRENT.write() {
        *g = w;
    }
}

pub(crate) fn now() -> Option<Weather> {
    CURRENT.read().ok().and_then(|g| g.clone())
}

/// The strip's text. The glyph says the sky, the rest says the numbers;
/// no colour emoji — a text glyph draws in the nav's own ink.
pub(crate) fn line(w: &Weather) -> String {
    let mut s = format!(
        "{} {}\u{00b0} \u{2191}{} \u{2193}{}",
        glyph(w.code),
        w.temp,
        w.hi,
        w.lo
    );
    if w.rain > 0 {
        s.push_str(&format!(" \u{2602}{}%", w.rain));
    }
    s
}

/// A text glyph for a WMO weather code (the 2-digit families).
pub(crate) fn glyph(code: u16) -> char {
    match code {
        0 => '\u{2600}',                 // ☀ clear
        1..=3 => '\u{2601}',             // ☁ mainly clear … overcast
        45 | 48 => '\u{2248}',           // ≈ fog
        51..=67 | 80..=82 => '\u{2602}', // ☂ drizzle, rain, showers
        71..=77 | 85 | 86 => '\u{2744}', // ❄ snow
        95..=99 => '\u{26a1}',           // ⚡ thunder
        _ => '\u{2601}',
    }
}

/// The geocoder's first hit: `(lat, lon, name, country_code)`. Pure.
pub(crate) fn parse_geo(body: &str) -> Option<(f64, f64, String, String)> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let hit = v.get("results")?.as_array()?.first()?;
    Some((
        hit.get("latitude")?.as_f64()?,
        hit.get("longitude")?.as_f64()?,
        hit.get("name")?.as_str()?.to_string(),
        hit.get("country_code")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string(),
    ))
}

/// One forecast body into a reading. Pure.
pub(crate) fn parse_forecast(body: &str, place: &str, unit: char) -> Option<Weather> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let cur = v.get("current")?;
    let daily = v.get("daily")?;
    let first = |k: &str| daily.get(k)?.as_array()?.first()?.as_f64();
    Some(Weather {
        place: place.to_string(),
        temp: cur.get("temperature_2m")?.as_f64()?.round() as i32,
        hi: first("temperature_2m_max")?.round() as i32,
        lo: first("temperature_2m_min")?.round() as i32,
        rain: first("precipitation_probability_max")
            .unwrap_or(0.0)
            .clamp(0.0, 100.0) as u8,
        code: cur.get("weather_code")?.as_u64()? as u16,
        unit,
    })
}

#[cfg(test)]
#[path = "navweather_tests.rs"]
mod tests;

//! The weather strip's worker: Open-Meteo's geocoder then forecast on a
//! short-lived thread with its own runtime (`modelfetch`'s pattern), a
//! disk cache beside the config keyed on the place, and the once-per-tick
//! drain that publishes the reading (`navweather::set`).
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use crate::navweather::{self, parse_forecast, parse_geo, State, Weather, TTL};

fn cache_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("weather.json"))
}

/// The cached reading when it is for `place` and younger than [`TTL`].
pub(crate) fn read_cache_at(path: &std::path::Path, place: &str, ttl: Duration) -> Option<Weather> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > 4096 || meta.modified().ok()?.elapsed().ok()? > ttl {
        return None;
    }
    let w: Weather = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    // The cache remembers what was ASKED, so a new place never serves the
    // old place's sky for an hour.
    let asked = std::fs::read_to_string(path.with_extension("place")).ok()?;
    // A reading without its hours is from before the card drew them: fetch.
    (asked == place && !w.hours.is_empty()).then_some(w)
}

fn write_cache(place: &str, w: &Weather) {
    let Some(path) = cache_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(body) = serde_json::to_string(w) {
        let _ = std::fs::write(path.with_extension("place"), place);
        let _ = std::fs::write(&path, body);
    }
}

async fn fetch(place: &str) -> Option<Weather> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("crew")
        .build()
        .ok()?;
    let geo = client
        .get("https://geocoding-api.open-meteo.com/v1/search")
        .query(&[
            ("name", place),
            ("count", "1"),
            ("language", "en"),
            ("format", "json"),
        ])
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;
    let (lat, lon, name, country) = parse_geo(&geo)?;
    let unit = if country == "US" { 'F' } else { 'C' };
    let temperature_unit = if unit == 'F' { "fahrenheit" } else { "celsius" };
    let body = client
        .get("https://api.open-meteo.com/v1/forecast")
        .query(&[
            ("latitude", lat.to_string()),
            ("longitude", lon.to_string()),
            ("current", "temperature_2m,weather_code".into()),
            ("hourly", "temperature_2m".into()),
            (
                "daily",
                "temperature_2m_max,temperature_2m_min,precipitation_probability_max".into(),
            ),
            ("timezone", "auto".into()),
            ("forecast_days", "2".into()),
            ("temperature_unit", temperature_unit.into()),
        ])
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;
    parse_forecast(&body, &name, unit)
}

/// Spawn the worker: cache first, the network only when stale. The answer
/// arrives on the channel (`None` when nothing could be found).
pub(crate) fn spawn(place: String) -> Receiver<Option<Weather>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        if let Some(w) = cache_path().and_then(|p| read_cache_at(&p, &place, TTL)) {
            let _ = tx.send(Some(w));
            return;
        }
        let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            let _ = tx.send(None);
            return;
        };
        let got = rt.block_on(fetch(&place));
        if let Some(w) = &got {
            write_cache(&place, w);
        }
        let _ = tx.send(got);
    });
    rx
}

impl crate::app::CrewApp {
    /// Once per tick: start a fetch when one is due, drain one that landed.
    /// Returns whether the strip changed. Cheap when nothing is set.
    pub(crate) fn tick_weather(&mut self, now_ms: u64) -> bool {
        let Some(place) = crate::navweatherplace::resolve(&self.config.weather_place) else {
            return false;
        };
        let mut changed = false;
        if now_ms >= self.weather_next {
            self.weather_fetch = Some(spawn(place.clone()));
            // Half the TTL: the cache answers the early ones for free.
            self.weather_next = now_ms + TTL.as_millis() as u64 / 2;
            // A reading stays on the card while its refresh is in flight;
            // only a card with nothing on it says it is looking.
            if navweather::now().is_none() {
                navweather::set(State::Looking(place.clone()));
                changed = true;
            }
        }
        let landed = self
            .weather_fetch
            .as_ref()
            .and_then(|rx| rx.try_recv().ok());
        match landed {
            Some(w) => {
                let next = match w {
                    Some(w) => State::Found(w),
                    None => {
                        self.set_status(format!("weather: nothing found for {place}"));
                        State::Missing(place)
                    }
                };
                let changed = next != navweather::state();
                navweather::set(next);
                self.weather_fetch = None;
                changed
            }
            None => changed,
        }
    }
}

#[cfg(test)]
#[path = "navweatherfetch_tests.rs"]
mod tests;

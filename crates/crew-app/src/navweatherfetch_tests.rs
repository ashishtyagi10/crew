use super::*;

const FORECAST: &str = r#"{"current":{"temperature_2m":23.6,"weather_code":61},"daily":{"temperature_2m_max":[27.1],"temperature_2m_min":[17.8],"precipitation_probability_max":[35]}}"#;

/// The cache serves only the place it was written for, and only while fresh.
#[test]
fn the_cache_is_keyed_on_the_place_and_the_ttl() {
    let dir = std::env::temp_dir().join(format!("crew-weather-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("weather.json");
    let w = parse_forecast(FORECAST, "Berlin", 'C').unwrap();
    std::fs::write(&path, serde_json::to_string(&w).unwrap()).unwrap();
    std::fs::write(path.with_extension("place"), "Berlin").unwrap();
    assert_eq!(read_cache_at(&path, "Berlin", TTL), Some(w));
    assert_eq!(read_cache_at(&path, "Paris", TTL), None, "another place");
    assert_eq!(
        read_cache_at(&path, "Berlin", Duration::ZERO),
        None,
        "stale"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

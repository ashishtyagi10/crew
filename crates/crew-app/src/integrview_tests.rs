use super::*;
use crew_plugin::integration::IntTool;

pub(crate) fn weather(auth: Auth) -> Integration {
    let tool = |name: &str, tier: Option<&str>| IntTool {
        name: name.into(),
        description: String::new(),
        method: "GET".into(),
        path: "/f".into(),
        query: Default::default(),
        body: None,
        input_schema: None,
        tier: tier.map(str::to_string),
    };
    Integration {
        name: "weather".into(),
        description: "Open-Meteo: the weather with no account and no key, anywhere on earth."
            .into(),
        base_url: "https://api.example.com".into(),
        auth,
        headers: Default::default(),
        tools: vec![
            tool("forecast", Some("read")),
            tool("subscribe_to_severe_weather_alerts", None),
            tool("unsubscribe", Some("reversible")),
        ],
    }
}

/// The credential line is the whole point: a manifest whose token is missing works until its
/// first call, and this says so before then.
#[test]
fn the_credential_is_named_and_its_absence_is_said_out_loud() {
    let bearer = weather(Auth::Bearer {
        env: "WEATHER_TOKEN".into(),
    });
    let out = listing(std::slice::from_ref(&bearer), &|_| false);
    assert!(
        out.contains("weather\n  WEATHER_TOKEN is NOT set \u{2014} calls will refuse"),
        "{out}"
    );
    let out = listing(&[bearer], &|e| e == "WEATHER_TOKEN");
    assert!(out.contains("weather\n  WEATHER_TOKEN is set"), "{out}");
    let out = listing(&[weather(Auth::None)], &|_| false);
    assert!(out.contains("weather\n  no credential needed"), "{out}");
}

/// Every tool with its tier — and a tool whose manifest names no tier is irreversible, the
/// same default the gate applies, so the listing never promises less asking than there is.
#[test]
fn every_tool_is_listed_with_the_tier_the_gate_will_use() {
    let out = listing(&[weather(Auth::None)], &|_| false);
    let row = |name: &str| out.lines().find(|l| l.contains(name)).unwrap().to_string();
    assert!(row("forecast").ends_with(" read"), "{}", row("forecast"));
    assert!(
        row("unsubscribe").ends_with(" reversible"),
        "{}",
        row("unsubscribe")
    );
    let cut = row("subscribe_t");
    assert!(cut.ends_with(" irreversible"), "{cut}");
    assert!(
        cut.contains('\u{2026}'),
        "a long name is cut, not wrapped: {cut}"
    );
    assert!(out.contains("1 integration(s) \u{b7} 3 tool(s)"), "{out}");
}

#[test]
fn every_row_fits_a_tiled_viewer() {
    let out = listing(
        &[weather(Auth::Header {
            name: "X-Api-Key".into(),
            env: "A_FAIRLY_LONG_VARIABLE_NAME".into(),
        })],
        &|_| false,
    );
    for line in out.lines().skip(1) {
        assert!(
            line.chars().count() <= ROW_W,
            "{} cols: {line:?}",
            line.chars().count()
        );
    }
}

#[test]
fn nothing_loaded_says_where_a_manifest_goes() {
    let out = listing(&[], &|_| true);
    assert!(out.contains("integrations/"), "{out}");
    assert!(out.contains("/reload"), "{out}");
    assert!(out.contains("weather.json"), "{out}");
}

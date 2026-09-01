//! What a manifest is allowed to say, and what it is not allowed to leave unsaid.
use super::*;

const WEATHER: &str = r#"{
  "name": "weather",
  "base_url": "https://api.example.com/v1",
  "auth": {"kind": "bearer", "env": "WEATHER_TOKEN"},
  "tools": [
    {"name": "forecast", "description": "the forecast for a place", "path": "/f/{city}", "tier": "read"},
    {"name": "alert", "description": "raise an alert", "method": "POST", "path": "/alert"}
  ]
}"#;

#[test]
fn a_manifest_becomes_a_server_of_tools() {
    let i = parse(WEATHER).expect("a usable manifest");
    assert_eq!(i.name, "weather");
    assert_eq!(i.tools.len(), 2);
    assert_eq!(
        i.auth,
        Auth::Bearer {
            env: "WEATHER_TOKEN".into()
        }
    );
    let tools = tools_of(std::slice::from_ref(&i));
    let labels: Vec<String> = tools
        .iter()
        .map(|t| format!("{}:{}", t.server, t.name))
        .collect();
    assert_eq!(labels, ["weather:forecast", "weather:alert"]);
    assert_eq!(
        tools[0].input_schema,
        serde_json::json!({"type": "object"}),
        "a tool with no schema still gets an object, never null"
    );
}

#[test]
fn a_tool_is_irreversible_unless_its_manifest_says_otherwise() {
    // The safety default, and the reason it is not "read": an integration nobody has thought
    // about must ask before it acts, exactly as an unknown MCP server does.
    let i = parse(WEATHER).unwrap();
    assert_eq!(i.tier_of("forecast"), super::super::tier::Tier::Read);
    assert_eq!(i.tier_of("alert"), super::super::tier::Tier::Irreversible);
    assert_eq!(
        i.tier_of("no-such-tool"),
        super::super::tier::Tier::Irreversible
    );
}

#[test]
fn a_misspelled_tier_is_not_permission() {
    let m = r#"{"name":"api","base_url":"http://a","tools":[{"name":"t","tier":"readonly"}]}"#;
    let i = parse(m).unwrap();
    assert_eq!(i.tier_of("t"), super::super::tier::Tier::Irreversible);
}

#[test]
fn a_manifest_that_could_not_work_is_dropped_rather_than_half_loaded() {
    for bad in [
        r#"{"name":"","base_url":"http://a","tools":[{"name":"t"}]}"#,
        r#"{"name":"api","base_url":"","tools":[{"name":"t"}]}"#,
        r#"{"name":"api","base_url":"http://a","tools":[]}"#,
        r#"{"name":"api","base_url":"http://a","tools":[{"name":"  "}]}"#,
        r#"{"name":"x","base_url":"http://a","tools":[{"name":"t"}]}"#,
        "not json at all",
    ] {
        // `x` is in that list for a reason of its own: a one-character server name cannot be
        // dialled as `server:tool` and is refused by the same rule agents are.
        assert!(parse(bad).is_none(), "{bad}");
    }
}

#[test]
fn there_is_no_field_that_holds_a_secret() {
    // A manifest gets copied between machines and committed to a repo. Every auth variant names
    // an environment variable, and a manifest that tries to inline a token simply does not parse
    // as one — the token field does not exist.
    let m = r#"{"name":"api","base_url":"http://a","auth":{"kind":"bearer","token":"sk-live-123"},
                "tools":[{"name":"t"}]}"#;
    assert!(parse(m).is_none(), "a bearer with no env is not an auth");
    // And the same manifest with the env named parses, so the refusal above is about the
    // token rather than about anything else in the file.
    let ok = r#"{"name":"api","base_url":"http://a","auth":{"kind":"bearer","env":"T"},
                "tools":[{"name":"t"}]}"#;
    assert!(parse(ok).is_some());
}

#[test]
fn a_project_manifest_replaces_a_user_one_of_the_same_name() {
    let dir = std::env::temp_dir().join(format!("crew-int-{}", std::process::id()));
    let proj = dir.join(".crew").join("integrations");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::write(
        proj.join("weather.json"),
        r#"{"name":"weather","base_url":"http://project","tools":[{"name":"forecast"}]}"#,
    )
    .unwrap();
    let all = load_at(&dir);
    let mine: Vec<&Integration> = all.iter().filter(|i| i.name == "weather").collect();
    assert_eq!(mine.len(), 1, "one weather, not two");
    assert_eq!(mine[0].base_url, "http://project");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_directory_that_is_not_there_is_no_integrations_rather_than_an_error() {
    assert!(load_dir(std::path::Path::new("/no/such/dir")).is_empty());
}

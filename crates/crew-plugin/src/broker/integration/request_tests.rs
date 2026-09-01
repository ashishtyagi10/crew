//! A URL assembled wrong is a request to somebody else's server, so this is the half that gets
//! the tests: substitution, encoding, credentials, and every message a failure sends back.
use super::*;

fn int(auth: Auth) -> Integration {
    Integration {
        name: "weather".into(),
        description: String::new(),
        base_url: "https://api.example.com/v1/".into(),
        auth,
        headers: BTreeMap::new(),
        tools: Vec::new(),
    }
}

fn tool(path: &str) -> IntTool {
    IntTool {
        name: "forecast".into(),
        description: String::new(),
        method: "GET".into(),
        path: path.into(),
        query: BTreeMap::new(),
        body: None,
        input_schema: None,
        tier: Some("read".into()),
    }
}

#[test]
fn a_path_placeholder_is_filled_from_the_arguments() {
    let r = build(
        &int(Auth::None),
        &tool("/forecast/{city}"),
        r#"{"city":"Oslo"}"#,
    )
    .unwrap();
    assert_eq!(r.url, "https://api.example.com/v1/forecast/Oslo");
    assert_eq!(r.method, "GET");
    assert!(r.body.is_none());
}

#[test]
fn the_base_urls_trailing_slash_does_not_double_up() {
    let r = build(&int(Auth::None), &tool("/f"), "{}").unwrap();
    assert_eq!(r.url, "https://api.example.com/v1/f");
}

#[test]
fn an_argument_is_encoded_rather_than_pasted() {
    // "San Francisco" unencoded is a broken URL; a value with a slash in it would otherwise
    // reach an endpoint the manifest never described.
    let r = build(
        &int(Auth::None),
        &tool("/f/{city}"),
        r#"{"city":"San Francisco/2"}"#,
    )
    .unwrap();
    assert_eq!(r.url, "https://api.example.com/v1/f/San%20Francisco%2F2");
}

#[test]
fn a_missing_argument_names_the_argument() {
    let e = build(&int(Auth::None), &tool("/f/{city}"), "{}").unwrap_err();
    assert!(e.contains("city"), "{e}");
}

#[test]
fn query_parameters_are_substituted_and_joined() {
    let mut t = tool("/f");
    t.query.insert("lat".into(), "{lat}".into());
    t.query.insert("units".into(), "metric".into());
    let r = build(&int(Auth::None), &t, r#"{"lat": 59.9}"#).unwrap();
    // A number argument is stringified rather than refused: the model answered correctly.
    assert_eq!(r.url, "https://api.example.com/v1/f?lat=59.9&units=metric");
}

#[test]
fn a_bearer_credential_comes_from_the_environment_and_never_from_the_file() {
    std::env::set_var("CREW_TEST_TOKEN", "s3cr3t");
    let r = build(
        &int(Auth::Bearer {
            env: "CREW_TEST_TOKEN".into(),
        }),
        &tool("/f"),
        "{}",
    )
    .unwrap();
    assert!(r
        .headers
        .contains(&("Authorization".into(), "Bearer s3cr3t".into())));
    std::env::remove_var("CREW_TEST_TOKEN");
}

#[test]
fn a_missing_credential_says_which_variable_to_set() {
    std::env::remove_var("CREW_TEST_ABSENT");
    let e = build(
        &int(Auth::Bearer {
            env: "CREW_TEST_ABSENT".into(),
        }),
        &tool("/f"),
        "{}",
    )
    .unwrap_err();
    assert!(e.contains("CREW_TEST_ABSENT"), "{e}");
    assert!(e.contains("not set"), "{e}");
}

#[test]
fn a_header_and_a_query_credential_land_where_they_belong() {
    std::env::set_var("CREW_TEST_KEY", "abc");
    let r = build(
        &int(Auth::Header {
            name: "X-Api-Key".into(),
            env: "CREW_TEST_KEY".into(),
        }),
        &tool("/f"),
        "{}",
    )
    .unwrap();
    assert!(r.headers.contains(&("X-Api-Key".into(), "abc".into())));
    let r = build(
        &int(Auth::Query {
            name: "key".into(),
            env: "CREW_TEST_KEY".into(),
        }),
        &tool("/f"),
        "{}",
    )
    .unwrap();
    assert_eq!(r.url, "https://api.example.com/v1/f?key=abc");
    std::env::remove_var("CREW_TEST_KEY");
}

#[test]
fn a_json_body_is_filled_and_keeps_the_arguments_own_types() {
    let mut t = tool("/alert");
    t.method = "POST".into();
    t.body = Some(serde_json::json!({"city": "{city}", "count": "{n}", "fixed": true}));
    let r = build(&int(Auth::None), &t, r#"{"city":"Oslo","n":3}"#).unwrap();
    let body: serde_json::Value = serde_json::from_str(&r.body.unwrap()).unwrap();
    assert_eq!(body["city"], "Oslo");
    assert_eq!(body["count"], 3, "a number stays a number, not \"3\"");
    assert_eq!(body["fixed"], true);
    assert!(r
        .headers
        .contains(&("Content-Type".into(), "application/json".into())));
}

#[test]
fn arguments_that_are_not_an_object_are_refused_with_a_reason() {
    let e = build(&int(Auth::None), &tool("/f"), "[1,2,3]").unwrap_err();
    assert!(e.contains("JSON object"), "{e}");
    let e = build(&int(Auth::None), &tool("/f"), "not json").unwrap_err();
    assert!(e.contains("not valid JSON"), "{e}");
}

use super::*;

fn tool(path: &str, schema: Option<serde_json::Value>) -> IntTool {
    IntTool {
        name: "forecast".into(),
        description: String::new(),
        method: "GET".into(),
        path: path.into(),
        query: [("latitude".to_string(), "{lat}".to_string())].into(),
        body: Some(serde_json::json!({"where": "{lon}", "note": "{lon} and {lat}"})),
        input_schema: schema,
        tier: None,
    }
}

fn schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "lat": {"type": "string", "description": "latitude, e.g. 59.91"},
            "lon": {"type": "string", "description": "longitude, e.g. 10.75"},
            "days": {"type": "integer", "description": "how many, up to 16"}
        },
        "required": ["lat", "lon"]
    })
}

fn given(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Every placeholder in the path, the query and the body — each once.
#[test]
fn the_placeholders_are_read_out_of_every_template() {
    let p = placeholders(&tool("/f/{city}/{lat}", None));
    let names: Vec<&str> = p.iter().map(String::as_str).collect();
    assert_eq!(names, ["city", "lat", "lon"]);
}

/// One message, every missing argument, what each is for, the optional ones too, and the
/// fact that nothing was given — the whole schema in the round the model would otherwise
/// spend learning one name.
#[test]
fn a_call_with_nothing_learns_everything_in_one_round() {
    let e = check("weather", &tool("/f", Some(schema())), &given(&[])).unwrap_err();
    assert_eq!(
        e,
        "weather:forecast is missing lat (latitude, e.g. 59.91), lon (longitude, e.g. 10.75); \
         it also takes days (how many, up to 16). The call had no arguments."
    );
}

/// Only what is still missing is named; a full call passes; a typo'd key is pointed at the
/// name it is nearest.
#[test]
fn a_partial_call_names_the_rest_and_a_typo_is_pointed_at_what_it_meant() {
    let t = tool("/f", Some(schema()));
    let e = check("weather", &t, &given(&[("lat", "1")])).unwrap_err();
    assert!(
        e.starts_with("weather:forecast is missing lon (longitude"),
        "{e}"
    );
    assert!(!e.contains("missing lat"), "{e}");
    assert!(!e.contains("no arguments"), "{e}");
    assert!(check("weather", &t, &given(&[("lat", "1"), ("lon", "2")])).is_ok());
    let e = check("weather", &t, &given(&[("latitude", "1"), ("lon", "2")])).unwrap_err();
    assert!(
        e.contains("\"latitude\" is not an argument \u{2014} did you mean \"lat\"?"),
        "{e}"
    );
}

/// No schema: the placeholders are the contract, and they are still all named at once.
#[test]
fn without_a_schema_the_placeholders_are_the_contract() {
    let e = check("weather", &tool("/f/{city}", None), &given(&[])).unwrap_err();
    assert_eq!(
        e,
        "weather:forecast is missing city, lat, lon. The call had no arguments."
    );
}

use super::*;

#[test]
fn parses_local_ids_and_crew_urls() {
    assert_eq!(
        parse_location("alpha"),
        Some(Location::Local("alpha".into()))
    );
    assert_eq!(parse_location(""), None);
    assert_eq!(
        parse_location("crew://host.example/main"),
        Some(Location::Remote {
            host: "host.example".into(),
            port: DEFAULT_RELAY_PORT,
            instance: "main".into(),
        })
    );
    assert_eq!(
        parse_location("crew://10.0.0.4:9000/build"),
        Some(Location::Remote {
            host: "10.0.0.4".into(),
            port: 9000,
            instance: "build".into(),
        })
    );
    // Malformed crew:// URLs are rejected, not silently downgraded.
    assert_eq!(parse_location("crew://host"), None); // no instance
    assert_eq!(parse_location("crew:///main"), None); // no host
    assert_eq!(parse_location("crew://host:notaport/main"), None);
}

#[test]
fn resolve_target_splits_pane_and_classifies_location() {
    assert_eq!(resolve_target("schema"), ("schema", Target::Local(None)));
    assert_eq!(
        resolve_target("schema@alpha"),
        ("schema", Target::Local(Some("alpha".into())))
    );
    assert_eq!(
        resolve_target("schema@crew://host/main"),
        (
            "schema",
            Target::Remote {
                host: "host".into(),
                port: DEFAULT_RELAY_PORT,
                instance: "main".into(),
            }
        )
    );
}

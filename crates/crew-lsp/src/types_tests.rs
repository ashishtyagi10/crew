use super::*;
use serde_json::json;

fn range(l0: u32, c0: u32, l1: u32, c1: u32) -> Range {
    Range {
        start: Position {
            line: l0,
            character: c0,
        },
        end: Position {
            line: l1,
            character: c1,
        },
    }
}

#[test]
fn a_definition_reply_parses_in_every_legal_shape() {
    let loc = json!({"uri": "file:///a.rs", "range": {"start": {"line": 1, "character": 2}, "end": {"line": 1, "character": 5}}});
    let want = Location {
        uri: "file:///a.rs".into(),
        range: range(1, 2, 1, 5),
    };
    assert_eq!(Location::parse_many(&loc), vec![want.clone()]);
    assert_eq!(
        Location::parse_many(&json!([loc.clone(), loc])),
        vec![want.clone(), want.clone()]
    );
    assert_eq!(Location::parse_many(&Value::Null), vec![]);
    // A LocationLink names its target twice; the SELECTION range is the
    // symbol itself, which is where a person wants to land.
    let link = json!({
        "targetUri": "file:///a.rs",
        "targetRange": {"start": {"line": 0, "character": 0}, "end": {"line": 9, "character": 0}},
        "targetSelectionRange": {"start": {"line": 1, "character": 2}, "end": {"line": 1, "character": 5}},
    });
    assert_eq!(Location::parse_many(&json!([link])), vec![want]);
}

#[test]
fn a_location_without_a_range_is_skipped_rather_than_invented() {
    assert!(Location::parse_many(&json!([{"uri": "file:///x"}])).is_empty());
}

#[test]
fn a_publish_diagnostics_fixture_parses_with_severities() {
    let params = json!({
        "uri": "file:///p/src/main.rs",
        "diagnostics": [
            {"range": {"start": {"line": 2, "character": 17}, "end": {"line": 2, "character": 20}},
             "severity": 1, "source": "rustc", "message": "mismatched types"},
            {"range": {"start": {"line": 4, "character": 8}, "end": {"line": 4, "character": 9}},
             "severity": 2, "message": "unused variable: `y`"},
            {"range": {"start": {"line": 6, "character": 0}, "end": {"line": 6, "character": 1}},
             "message": "no severity given"},
            {"range": {"start": {"line": 7, "character": 0}, "end": {"line": 7, "character": 1}},
             "severity": 4, "message": "a hint"}
        ]
    });
    let (uri, list) = Diagnostic::parse_publish(&params).unwrap();
    assert_eq!(uri, "file:///p/src/main.rs");
    assert_eq!(list.len(), 4);
    assert_eq!(list[0].severity, Severity::Error);
    assert_eq!(list[0].source.as_deref(), Some("rustc"));
    assert_eq!(list[0].range, range(2, 17, 2, 20));
    assert_eq!(list[1].severity, Severity::Warning);
    assert!(list[1].source.is_none());
    assert_eq!(
        list[2].severity,
        Severity::Error,
        "omitted severity reads as error"
    );
    assert_eq!(list[3].severity, Severity::Hint);
    assert_eq!(list.iter().filter(|d| d.is_error()).count(), 2);
}

#[test]
fn a_publish_with_no_list_is_an_empty_list_and_no_uri_is_none() {
    let (_, list) = Diagnostic::parse_publish(&json!({"uri": "file:///x"})).unwrap();
    assert!(list.is_empty());
    assert!(Diagnostic::parse_publish(&json!({"diagnostics": []})).is_none());
}

#[test]
fn severity_labels_are_the_words_a_person_reads() {
    assert_eq!(Severity::from_code(Some(1)).label(), "error");
    assert_eq!(Severity::from_code(Some(2)).label(), "warning");
    assert_eq!(Severity::from_code(Some(3)).label(), "info");
    assert_eq!(Severity::from_code(None).label(), "error");
}

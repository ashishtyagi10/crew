use serde_json::json;

use super::{invocation, parse};

#[test]
fn valid_arguments_and_an_empty_string_read_as_they_always_did() {
    assert_eq!(parse(r#"{"path":"a.rs"}"#), Ok(json!({"path": "a.rs"})));
    assert_eq!(parse(""), Ok(json!({})));
    assert_eq!(parse("  \n"), Ok(json!({})));
}

#[test]
fn a_trailing_comma_is_dropped() {
    assert_eq!(parse(r#"{"path":"a.rs",}"#), Ok(json!({"path": "a.rs"})));
    assert_eq!(
        parse("{\"paths\": [\"a\", \"b\",],\n}"),
        Ok(json!({"paths": ["a", "b"]}))
    );
}

#[test]
fn a_whole_object_then_junk_is_the_object() {
    assert_eq!(parse(r#"{"path":"a.rs"}}"#), Ok(json!({"path": "a.rs"})));
    assert_eq!(
        parse(r#"{"path":"a.rs"}</tool_call>"#),
        Ok(json!({"path": "a.rs"}))
    );
    assert_eq!(parse("{\"n\":1,}\n```"), Ok(json!({"n": 1})));
}

#[test]
fn two_objects_run_together_are_refused_rather_than_one_picked() {
    assert!(parse(r#"{"path":"a.rs"}{"path":"b.rs"}"#).is_err());
    assert!(parse(r#"{"path":"a.rs"}, {"path":"b.rs"}"#).is_err());
}

#[test]
fn a_cut_or_mis_quoted_object_says_why_and_shows_what_was_sent() {
    let why = parse(r#"{"path": "a.rs""#).unwrap_err();
    assert!(why.starts_with("EOF while parsing an object"), "{why}");
    assert!(why.ends_with(r#" — {"path": "a.rs""#), "{why}");
    let why = parse("{'path': 'a.rs'}").unwrap_err();
    assert!(why.starts_with("key must be a string"), "{why}");
    // Never closed for the model: a string cut short has no one reading.
    assert!(parse(r#"{"path": "a.r"#).is_err());
}

#[test]
fn what_is_shown_back_is_one_line_and_at_most_120_characters() {
    let long = format!("{{\"content\":\n\"{}", "x".repeat(500));
    let why = parse(&long).unwrap_err();
    let shown = why.split(" \u{2014} ").nth(1).unwrap();
    assert!(!why.contains('\n'), "{why}");
    assert_eq!(shown.chars().count(), 121, "120 and the ellipsis: {shown}");
    assert!(shown.starts_with("{\"content\": \"xxx"), "{shown}");
    assert!(shown.ends_with('\u{2026}'), "{shown}");
}

#[test]
fn an_invocation_carries_why_and_an_empty_object_in_place_of_the_arguments() {
    let bad = invocation("c".into(), "fs__read".into(), r#"{"path": "a"#);
    assert_eq!(bad.input, json!({}));
    assert!(bad.bad_args.is_some());
    let good = invocation("c".into(), "fs__read".into(), r#"{"path":"a",}"#);
    assert_eq!(good.input, json!({"path": "a"}));
    assert_eq!(good.bad_args, None);
}

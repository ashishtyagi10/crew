use super::*;

#[test]
fn extracts_text_parts_in_order() {
    let raw = concat!(
        r#"{"type":"step","timestamp":1}"#,
        "\n",
        r#"{"type":"message","part":{"type":"text","text":"PONG"}}"#,
    );
    assert_eq!(opencode_json(raw), "PONG");
}

#[test]
fn joins_multiple_text_chunks() {
    let raw = concat!(
        r#"{"type":"text","text":"Hello "}"#,
        "\n",
        r#"{"type":"text","text":"world"}"#,
    );
    assert_eq!(opencode_json(raw), "Hello world");
}

#[test]
fn ignores_non_json_noise_lines() {
    let raw = concat!(
        "[08:29:02.746] ERROR (#10950): failed {\n",
        r#"{"type":"text","text":"ok"}"#,
    );
    assert_eq!(opencode_json(raw), "ok");
}

#[test]
fn surfaces_error_events_when_no_text() {
    let raw = r#"{"type":"error","error":{"name":"ProviderAuthError","data":{"message":"API key is missing"}}}"#;
    assert_eq!(opencode_json(raw), "[opencode error] API key is missing");
}

#[test]
fn error_falls_back_to_name() {
    let raw = r#"{"type":"error","error":{"name":"UnknownError"}}"#;
    assert_eq!(opencode_json(raw), "[opencode error] UnknownError");
}

#[test]
fn empty_or_unparseable_returns_trimmed_raw() {
    assert_eq!(opencode_json("  not json  "), "not json");
}

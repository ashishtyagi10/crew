use serde_json::json;

use super::*;

#[test]
fn a_discover_result_picks_the_newest_common_modern_version() {
    let r = json!({"supportedVersions": ["2025-11-25", "2026-07-28"]});
    assert_eq!(classify(Ok(&r)), Probe::Modern("2026-07-28".into()));
    let r = json!({"supportedVersions": ["2030-01-01"]});
    assert!(matches!(classify(Ok(&r)), Probe::Refused(e) if e.contains("2030-01-01")));
}

#[test]
fn only_the_unsupported_version_error_marks_a_modern_server() {
    let ok = json!({"code": -32022, "data": {"supported": ["2026-07-28"]}});
    assert_eq!(classify(Err(&ok)), Probe::Modern("2026-07-28".into()));
    let none = json!({"code": -32022, "data": {"supported": ["2025-11-25"]}});
    assert!(matches!(classify(Err(&none)), Probe::Refused(_)));
    for code in [-32601, -32602, -32000] {
        assert_eq!(classify(Err(&json!({"code": code}))), Probe::Legacy);
    }
}

#[test]
fn meta_goes_on_modern_requests_only() {
    let p = stamp(json!({"cursor": "c"}), "2026-07-28");
    assert_eq!(p["cursor"], "c");
    assert_eq!(
        p["_meta"]["io.modelcontextprotocol/protocolVersion"],
        "2026-07-28"
    );
    assert_eq!(
        p["_meta"]["io.modelcontextprotocol/clientCapabilities"],
        json!({})
    );
    assert_eq!(
        p["_meta"]["io.modelcontextprotocol/clientInfo"]["name"],
        "crew"
    );
    assert!(stamp(json!({}), "2025-11-25").get("_meta").is_none());
}

#[test]
fn legacy_answers_crew_speaks_are_kept_and_others_named() {
    for v in LEGACY {
        assert_eq!(accept_legacy(Some(v)).unwrap(), *v);
    }
    assert_eq!(accept_legacy(None).unwrap(), "2025-11-25");
    let e = accept_legacy(Some("2026-07-28")).unwrap_err();
    assert!(e.contains("answered MCP 2026-07-28"), "{e}");
}

#[test]
fn only_complete_results_are_tool_output() {
    assert!(complete("x", json!({"content": []})).is_ok());
    assert!(complete("x", json!({"resultType": "complete"})).is_ok());
    let e = complete("tools/call", json!({"resultType": "input_required"})).unwrap_err();
    assert!(e.contains("input_required"), "{e}");
    assert!(complete("x", json!({"resultType": "later"})).is_err());
}

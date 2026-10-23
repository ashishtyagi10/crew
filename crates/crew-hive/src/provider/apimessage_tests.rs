use super::*;

#[test]
fn a_rejected_key_says_so_and_says_what_to_do() {
    let body = r#"{"error":{"message":"Incorrect API key provided: sk-abc***.","type":"invalid_request_error"}}"#;
    let m = api_message(body);
    assert!(m.contains("rejected the key"), "{m}");
    assert!(m.contains("Incorrect API key provided"), "{m}");
    assert!(m.contains("/model"), "the way to fix it: {m}");
    assert!(!m.contains('{'), "no JSON envelope: {m}");
}

#[test]
fn other_errors_keep_their_sentence_without_the_envelope() {
    let body = r#"{"error":{"message":"This model is overloaded","type":"server_error"}}"#;
    let m = api_message(body);
    assert_eq!(m, "api error: This model is overloaded");
}

/// A body that is not the expected shape must still show the error. The
/// worst outcome here is hiding a failure behind a parse miss.
#[test]
fn an_unparseable_body_is_shown_not_swallowed() {
    let m = api_message("<html>502 Bad Gateway</html>");
    assert!(m.contains("502 Bad Gateway"), "{m}");
    assert!(!api_message("").is_empty());
}

/// Multi-line bodies collapse: an error is one line, however it arrived.
#[test]
fn a_sprawling_body_becomes_one_bounded_line() {
    let body = format!("{{\"error\":{{\"message\":\"{}\"}}}}", "x ".repeat(400));
    let m = api_message(&body);
    assert!(!m.contains('\n'), "{m}");
    assert!(m.chars().count() < 240, "len {}", m.chars().count());
}

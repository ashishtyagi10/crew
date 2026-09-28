//! `ProviderError::is_transient`: which failures earn a swarm task a second
//! run. Each case is a shape the providers really produce — `wire_error`'s
//! sentences, `status::sentence`'s, and the envelopes `retry_tests` reads.
use super::super::ProviderError;

#[test]
fn the_wire_failing_passes() {
    for said in [
        "api.example.com went quiet \u{2014} read timed out",
        "could not connect to api.example.com (connection refused)",
        "api.example.com dropped the response mid-stream (connection reset by peer)",
    ] {
        assert!(ProviderError::Http(said.into()).is_transient(), "{said}");
    }
}

#[test]
fn a_status_passes_when_the_host_is_busy_or_failing() {
    let status = |s: &str| ProviderError::Status(s.into()).is_transient();
    assert!(status("HTTP 429 from api.example.com: Too Many Requests"));
    assert!(status(
        "HTTP 502 from dashscope-intl.aliyuncs.com: Bad Gateway"
    ));
    assert!(status("HTTP 529 from api.anthropic.com"));
    assert!(!status("HTTP 404 from models.github.ai: Not Found"));
    assert!(!status("HTTP 401 from api.example.com: Unauthorized"));
    assert!(!status(
        "HTTP 200 from api.example.com: the reply is not JSON"
    ));
}

#[test]
fn an_envelope_passes_when_it_names_a_rate_limit_or_an_overload() {
    let api = |s: &str| ProviderError::Api(s.into()).is_transient();
    assert!(api(
        r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#
    ));
    assert!(api(
        r#"{"error":{"message":"Provider returned error","code":429}}"#
    ));
    assert!(api(
        r#"{"error":{"message":"You exceeded your rate limit"}}"#
    ));
    assert!(!api("model qwen-maxx does not exist"));
    assert!(!api(
        r#"{"error":{"message":"Incorrect API key provided: sk-abc***.","type":"invalid_request_error"}}"#
    ));
    assert!(!api(
        r#"{"type":"error","error":{"type":"invalid_request_error","message":"max_tokens: too big"}}"#
    ));
}

#[test]
fn what_fails_the_same_way_every_time_does_not_pass() {
    assert!(!ProviderError::MissingKey("OPENROUTER_API_KEY").is_transient());
    assert!(!ProviderError::Decode("missing usage".into()).is_transient());
}

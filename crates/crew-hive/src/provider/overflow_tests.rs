//! Which errors are a request too long for the model's context, on the bodies
//! the hosts send, and the ones that must not be taken for one.
use super::says_context_overflow;
use crate::provider::ProviderError;

const DASHSCOPE: &str = r#"{"error":{"message":"Range of input length should be [1, 30720]","code":"invalid_parameter_error"}}"#;
const OPENAI: &str = r#"{"error":{"message":"This model's maximum context length is 32768 tokens. However, you requested 40100 tokens.","type":"invalid_request_error","code":"context_length_exceeded"}}"#;
const ANTHROPIC: &str = r#"{"type":"error","error":{"type":"invalid_request_error","message":"prompt is too long: 210000 tokens > 200000 maximum"}}"#;

#[test]
fn each_hosts_context_error_is_an_overflow() {
    for body in [DASHSCOPE, OPENAI, ANTHROPIC] {
        assert!(
            ProviderError::Api(body.into()).is_context_overflow(),
            "{body}"
        );
    }
}

/// The code alone says it when the sentence does not.
#[test]
fn the_envelopes_code_is_read_too() {
    let body = r#"{"error":{"message":"Bad request","code":"context_length_exceeded"}}"#;
    assert!(ProviderError::Api(body.into()).is_context_overflow());
}

#[test]
fn other_400s_and_transport_errors_are_not() {
    let bad_param = r#"{"error":{"message":"<400> InternalError.Algo.InvalidParameter: The tool call is not supported.","code":"invalid_parameter_error"}}"#;
    let key = r#"{"error":{"message":"Incorrect API key provided: sk-abc***.","type":"invalid_request_error"}}"#;
    for body in [bad_param, key] {
        assert!(
            !ProviderError::Api(body.into()).is_context_overflow(),
            "{body}"
        );
    }
    let http = ProviderError::Http("maximum context length exceeded".into());
    assert!(
        !http.is_context_overflow(),
        "the words of a transport error"
    );
    assert!(!ProviderError::Decode("prompt is too long".into()).is_context_overflow());
}

/// The relay sees only the error's display, hint and all.
#[test]
fn the_display_of_one_is_still_one_and_of_another_is_not() {
    let said = ProviderError::Api(DASHSCOPE.into()).to_string();
    assert!(says_context_overflow(&said), "{said}");
    assert!(says_context_overflow("Prompt is too long"));
    let key = ProviderError::Api(r#"{"error":{"message":"Invalid API key"}}"#.into());
    assert!(!says_context_overflow(&key.to_string()));
    assert!(!says_context_overflow(
        "planner: api call timed out after 180s (raise CREW_BROKER_TIMEOUT_MS?)"
    ));
}

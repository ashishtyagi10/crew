use super::{why, Walk};
use crate::provider::{Completion, ProviderError};

fn reply() -> Completion {
    Completion {
        text: "hi".into(),
        ..Default::default()
    }
}

/// The first model answered: nothing was skipped, and nothing says so.
#[test]
fn a_first_model_that_answers_skipped_nothing() {
    let c = Walk::default().answered("qwen-flash", reply()).unwrap();
    assert_eq!(c.fell_back, "");
    assert_eq!(c.model, "qwen-flash");
}

/// A 404 in the provider's own envelope is said as its sentence, named by
/// the model that failed — and the reply is the model that answered's.
#[test]
fn a_skipped_model_is_named_with_its_reason() {
    let mut walk = Walk::default();
    let body =
        r#"{"error":{"message":"The model `qwen-nope` does not exist","code":"model_not_found"}}"#;
    walk.failed("qwen-nope", ProviderError::Api(body.into()));
    let c = walk.answered("qwen-flash", reply()).unwrap();
    assert_eq!(
        c.fell_back,
        "qwen-nope: The model `qwen-nope` does not exist"
    );
    assert_eq!(c.model, "qwen-flash");
}

/// Two models down, both are named, in the order they fell.
#[test]
fn every_skipped_model_is_named_in_order() {
    let mut walk = Walk::default();
    let gateway = "HTTP 502 from dashscope-intl.aliyuncs.com: Bad Gateway";
    walk.failed("a", ProviderError::Status(gateway.into()));
    walk.failed("b", ProviderError::Http("could not connect to x.ai".into()));
    let c = walk.answered("c", reply()).unwrap();
    assert_eq!(
        c.fell_back,
        format!("a: {gateway}; b: could not connect to x.ai")
    );
}

/// Nothing answered: the error is the LAST model's, as it always was.
#[test]
fn a_chain_with_no_answer_reports_its_last_error() {
    let mut walk = Walk::default();
    walk.failed("a", ProviderError::Http("first".into()));
    walk.failed("b", ProviderError::Http("second".into()));
    let e = walk.exhausted().unwrap_err();
    assert!(e.to_string().contains("second"), "{e}");
}

/// A provider that answers with a page of prose is cut to one short line:
/// the reason rides in a status line, not a card.
#[test]
fn a_long_reason_is_clipped_to_one_short_line() {
    let body = format!("{{\"error\":{{\"message\":\"{}\"}}}}", "word\n".repeat(60));
    let w = why(&ProviderError::Api(body));
    assert!(!w.contains('\n'), "{w}");
    assert!(w.ends_with('\u{2026}'), "{w}");
    assert!(w.chars().count() <= 81, "{} chars: {w}", w.chars().count());
}

//! A failure of the ACCOUNT ends the walk down the chain; a failure of one model does not.
use std::sync::atomic::Ordering;
use std::time::Duration;

use super::tests::one_shot_server;
use super::OpenRouterProvider;
use crate::provider::fellback::account_wide;
use crate::provider::{CompletionRequest, Provider, ProviderError};

fn api(body: &str) -> ProviderError {
    ProviderError::Api(body.into())
}

#[test]
fn which_failures_belong_to_the_account() {
    let refused = r#"{"error":{"message":"Incorrect API key provided: sk-***","type":"invalid_request_error","code":"invalid_api_key"}}"#;
    let unpaid = r#"{"error":{"message":"You exceeded your current quota, please check your plan and billing details.","type":"insufficient_quota","code":"insufficient_quota"}}"#;
    let deepseek = r#"{"error":{"message":"Insufficient Balance","type":"unknown_error"}}"#;
    let arrears = r#"{"error":{"message":"Access denied, please make sure your account is in good standing.","code":"Arrearage"}}"#;
    for e in [api(refused), api(unpaid), api(deepseek), api(arrears)] {
        assert!(account_wide(&e), "{e}");
    }
    assert!(account_wide(&ProviderError::MissingKey("X")));
    assert!(account_wide(&ProviderError::Status(
        "HTTP 401 from api.x.com: Unauthorized".into()
    )));
    // A free rung still answers these; per-model quotas and access are the model's.
    let credits = r#"{"error":{"message":"Insufficient credits. Add more using https://openrouter.ai/settings/credits","code":402}}"#;
    let gemini = r#"{"error":{"message":"You exceeded your current quota, please check your plan.","status":"RESOURCE_EXHAUSTED","code":429}}"#;
    let unpurchased = r#"{"error":{"message":"Access to model denied. Please make sure you are eligible for using the model.","code":"AccessDenied.Unpurchased"}}"#;
    for e in [api(credits), api(gemini), api(unpurchased)] {
        assert!(!account_wide(&e), "{e}");
    }
    assert!(!account_wide(&ProviderError::Http(
        "connection reset".into()
    )));
}

fn answering(status: &str, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

async fn ask(addr: std::net::SocketAddr) -> Result<crate::provider::Completion, ProviderError> {
    let p = OpenRouterProvider::new("k".into())
        .with_endpoint(format!("http://{addr}/v1/chat/completions"))
        .with_fallbacks(vec!["second".into(), "third".into()]);
    let req = CompletionRequest {
        model: "first".into(),
        prompt: "hi".into(),
        max_tokens: 8,
        ..Default::default()
    };
    tokio::time::timeout(Duration::from_secs(10), p.complete(req))
        .await
        .expect("must not hang")
}

/// A refused key: one request, not one per rung, and the key's own hint.
#[tokio::test]
async fn a_refused_key_asks_once_not_once_per_rung() {
    let body = r#"{"error":{"message":"Incorrect API key provided","code":"invalid_api_key"}}"#;
    let (addr, accepted) = one_shot_server(answering("401 Unauthorized", body), 3);
    let e = ask(addr).await.expect_err("refused");
    assert!(e.to_string().contains("rejected the key"), "{e}");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        accepted.load(Ordering::SeqCst),
        1,
        "the chain stopped at the account"
    );
}

/// A model's own failure still walks on to the next rung.
#[tokio::test]
async fn a_models_own_failure_still_walks_the_chain() {
    let body = r#"{"error":{"message":"Insufficient credits.","code":402}}"#;
    let (addr, accepted) = one_shot_server(answering("402 Payment Required", body), 3);
    let _ = ask(addr).await.expect_err("no rung answers");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(accepted.load(Ordering::SeqCst), 3, "every rung was asked");
}

/// OpenAI's answer for a model it has shut down is the model's failure, not
/// the account's: the `openai` chain must walk past a dead `gpt-4.1` to its
/// last rung, `gpt-6-luna`, rather than stop there.
#[tokio::test]
async fn a_shut_down_model_walks_on_to_the_next_rung() {
    let body = r#"{"error":{"message":"The model `gpt-4.1` does not exist or you do not have access to it.","type":"invalid_request_error","param":null,"code":"model_not_found"}}"#;
    assert!(!account_wide(&api(body)));
    let (addr, accepted) = one_shot_server(answering("404 Not Found", body), 3);
    let _ = ask(addr).await.expect_err("no rung answers");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(accepted.load(Ordering::SeqCst), 3, "every rung was asked");
}

/// The body this provider builds for a Luna tool turn, as OpenAI's host
/// receives it, carries the `reasoning_effort: none` Luna's tools need. A
/// socket fake cannot show this: the seam keys on the host `api.openai.com`,
/// which no loopback server is, so the built body goes through the seam here.
#[test]
fn the_built_luna_tool_body_asks_openai_for_no_effort() {
    let req = CompletionRequest {
        model: "gpt-6-luna".into(),
        prompt: "hi".into(),
        max_tokens: 8,
        tools: vec![crate::provider::ToolDef {
            name: "sys_run".into(),
            description: "run".into(),
            input_schema: serde_json::json!({"type": "object"}),
        }],
        ..Default::default()
    };
    let built = |model: &str| {
        let messages = super::build_messages(&req);
        super::super::tokencap::for_endpoint(
            "https://api.openai.com/v1/chat/completions",
            super::build_body(model, &req, &messages, false, Vec::new()),
        )
    };
    let luna = built("gpt-6-luna");
    assert_eq!(luna["reasoning_effort"], "none", "{luna}");
    assert!(luna["tools"].as_array().is_some_and(|t| !t.is_empty()));
    let head = built("gpt-5.5");
    assert!(head.get("reasoning_effort").is_none(), "{head}");
}

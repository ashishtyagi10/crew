//! The OpenAI-compatible chat-completions HTTP layer shared by every
//! provider that speaks that shape (OpenRouter, Alibaba DashScope, …):
//! transient-error retry with Retry-After honouring, and response parsing.
use serde::Deserialize;

use super::{Completion, ProviderError};

/// How many times to retry one model on a transient error before the chain
/// advances to the next model (kept low because the fallback chain adds breadth).
const MAX_RETRIES: u32 = 2;

/// Seconds to wait before retrying, or `None` to not retry. A call is treated as
/// transiently retryable when the HTTP status is 429/5xx *or* the body carries an
/// OpenRouter-wrapped upstream rate-limit error (it returns those as a 200 with
/// an `error` object of `"code":429`). Honors an explicit `Retry-After` header or
/// the body's `retry_after_seconds`, else backs off exponentially; clamped so a
/// hung retry loop can't outlast the agent call's own timeout.
pub(super) fn retry_delay(
    status: u16,
    retry_after_hdr: Option<u64>,
    body: &str,
    attempt: u32,
) -> Option<u64> {
    let transient = status == 429
        || (500..600).contains(&status)
        || body.contains("\"code\":429")
        || body.contains("rate-limit")
        || body.contains("rate limit");
    if !transient {
        return None;
    }
    let body_hint = body
        .split("retry_after_seconds\":")
        .nth(1)
        .and_then(|s| s.split([',', '}']).next())
        .and_then(|s| s.trim().parse::<f64>().ok())
        .map(|f| f.ceil() as u64);
    Some(
        retry_after_hdr
            .or(body_hint)
            .unwrap_or(1u64 << attempt)
            .clamp(1, 8),
    )
}

/// One model's request with transient-error retry (see [`retry_delay`]).
pub(super) async fn request_with_retry(
    client: &reqwest::Client,
    endpoint: &str,
    key: &str,
    body: &serde_json::Value,
) -> Result<Completion, ProviderError> {
    let mut attempt = 0u32;
    loop {
        let resp = client
            .post(endpoint)
            .header("authorization", format!("Bearer {key}"))
            .header("content-type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        let status = resp.status().as_u16();
        let retry_after_hdr = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok());
        let text = resp
            .text()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        if attempt < MAX_RETRIES {
            if let Some(wait) = retry_delay(status, retry_after_hdr, &text, attempt) {
                attempt += 1;
                tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                continue;
            }
        }
        return parse_response(&text);
    }
}

#[derive(Deserialize, Default)]
struct Msg {
    #[serde(default)]
    content: String,
}

#[derive(Deserialize)]
struct Choice {
    #[serde(default)]
    message: Msg,
}

#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

#[derive(Deserialize)]
struct ApiResp {
    #[serde(default)]
    choices: Vec<Choice>,
    usage: Option<Usage>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

/// Decode an OpenAI-shape chat-completions body into a [`Completion`].
pub(super) fn parse_response(body: &str) -> Result<Completion, ProviderError> {
    let r: ApiResp =
        serde_json::from_str(body).map_err(|e| ProviderError::Decode(e.to_string()))?;
    if r.error.is_some() {
        return Err(ProviderError::Api(body.to_string()));
    }
    let text = r
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();
    let usage = r
        .usage
        .ok_or_else(|| ProviderError::Decode("missing usage".into()))?;
    Ok(Completion {
        text,
        input_tokens: usage.prompt_tokens,
        output_tokens: usage.completion_tokens,
    })
}

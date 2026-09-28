//! Transient-failure retry, one rule for every HTTP provider.
//!
//! A busy host is not a broken one. A 429 (rate limit), a 5xx, and
//! Anthropic's 529 (`overloaded_error`, what it answers at peak hours)
//! usually clear within seconds, and failing the turn on the first one threw
//! the question away. The OpenAI-compatible path retried and the Anthropic
//! path did not; both now ask [`again`] after a failed call.
use std::borrow::Cow;
use std::time::Duration;

/// How many times one request is retried on a transient error before the
/// failure stands. Kept low: a person is waiting, and OpenRouter's fallback
/// chain adds breadth on top.
const MAX_RETRIES: u32 = 2;

/// Seconds to wait before retrying, or `None` to not retry. A call is treated as
/// transiently retryable when the HTTP status is 429/5xx (529 included) *or* the
/// error names a rate limit or an overload (see [`transient`]): OpenRouter's
/// upstream rate-limit error (returned as a 200 with an `error` object of
/// `"code":429`), or Anthropic's `rate_limit_error` / `overloaded_error`,
/// which also arrive as an `error` event inside a 200 stream. Honors an
/// explicit `Retry-After` header or the body's `retry_after_seconds`, else
/// backs off exponentially; capped at 8 s so a hung retry loop can't outlast
/// the agent call's own timeout. An explicit 0 is the server saying "now" and
/// is taken at its word (it used to be raised to a second); the backoff alone
/// starts at one.
pub(super) fn retry_delay(
    status: u16,
    retry_after_hdr: Option<u64>,
    body: &str,
    attempt: u32,
) -> Option<u64> {
    if !transient(status, body) {
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
            .min(8),
    )
}

/// Whether an answer is a failure that passes. The status decides first; the
/// words decide only for what is already known to be an error.
///
/// A 2xx is an error only when it says so, as a JSON object with a top-level
/// `error` — OpenRouter's wrapped upstream 429, Anthropic's in-stream `error`
/// event — and then only that object is read. The words used to be looked
/// for in the whole body whatever the status, so a REPLY that talked about
/// rate limits ("add a rate limit to the endpoint") read as one: it was
/// asked for again, and billed, twice more before the answer stood.
fn transient(status: u16, body: &str) -> bool {
    if status == 429 || (500..600).contains(&status) {
        return true;
    }
    let said: Cow<str> = if (200..300).contains(&status) {
        match error_of(body) {
            Some(e) => Cow::Owned(e),
            None => return false,
        }
    } else {
        Cow::Borrowed(body)
    };
    [
        "\"code\":429",
        "\"overloaded_error\"",
        "\"rate_limit_error\"",
        "rate-limit",
        "rate limit",
    ]
    .iter()
    .any(|p| said.contains(p))
}

/// The top-level `error` of a JSON object body, as compact JSON text — or
/// `None` for a reply, a body that is not JSON, and `"error": null`.
fn error_of(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let e = v.as_object()?.get("error").filter(|e| !e.is_null())?;
    Some(e.to_string())
}

/// The `Retry-After` header in whole seconds, when the server sent one.
pub(super) fn retry_after(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
}

/// How long to wait before trying once more, counting the try — or `None`
/// when the failure is not transient (see [`retry_delay`]) or the retries
/// are spent, and it should stand.
pub(super) fn again(
    status: u16,
    retry_after_hdr: Option<u64>,
    body: &str,
    attempt: &mut u32,
) -> Option<Duration> {
    if *attempt >= MAX_RETRIES {
        return None;
    }
    let wait = retry_delay(status, retry_after_hdr, body, *attempt)?;
    *attempt += 1;
    Some(Duration::from_secs(wait))
}

#[cfg(test)]
#[path = "retry_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "retryreply_tests.rs"]
mod reply_tests;

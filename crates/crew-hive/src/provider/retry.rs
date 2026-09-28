//! Transient-failure retry, one rule for every HTTP provider.
//!
//! A busy host is not a broken one. A 429 (rate limit), a 5xx, and
//! Anthropic's 529 (`overloaded_error`, what it answers at peak hours)
//! usually clear within seconds, and failing the turn on the first one threw
//! the question away. The OpenAI-compatible path retried and the Anthropic
//! path did not; both now ask [`again`] after a failed call.
use std::time::Duration;

/// How many times one request is retried on a transient error before the
/// failure stands. Kept low: a person is waiting, and OpenRouter's fallback
/// chain adds breadth on top.
const MAX_RETRIES: u32 = 2;

/// Seconds to wait before retrying, or `None` to not retry. A call is treated as
/// transiently retryable when the HTTP status is 429/5xx (529 included) *or* the
/// body names a rate limit or an overload: OpenRouter's upstream rate-limit
/// error (returned as a 200 with an `error` object of `"code":429`), or
/// Anthropic's `rate_limit_error` / `overloaded_error`, which also arrive as
/// an `error` event inside a 200 stream. Honors an explicit `Retry-After`
/// header or the body's `retry_after_seconds`, else backs off exponentially;
/// capped at 8 s so a hung retry loop can't outlast the agent call's own
/// timeout. An explicit 0 is the server saying "now" and is taken at its
/// word (it used to be raised to a second); the backoff alone starts at one.
pub(super) fn retry_delay(
    status: u16,
    retry_after_hdr: Option<u64>,
    body: &str,
    attempt: u32,
) -> Option<u64> {
    let transient = status == 429
        || (500..600).contains(&status)
        || body.contains("\"code\":429")
        || body.contains("\"overloaded_error\"")
        || body.contains("\"rate_limit_error\"")
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
            .min(8),
    )
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

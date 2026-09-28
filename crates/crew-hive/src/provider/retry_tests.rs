use super::*;

const OVERLOADED: &str =
    r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#;

#[test]
fn anthropic_busy_and_rate_limited_bodies_are_transient_under_any_status() {
    assert_eq!(retry_delay(529, None, OVERLOADED, 0), Some(1));
    // The same errors as an `error` event inside a 200 stream.
    assert_eq!(retry_delay(200, None, OVERLOADED, 1), Some(2));
    let limited = r#"{"type":"error","error":{"type":"rate_limit_error","message":"Number of requests has exceeded your per-minute rate limit"}}"#;
    assert_eq!(retry_delay(200, None, limited, 0), Some(1));
    // A plain bad request is not retried.
    let bad = r#"{"type":"error","error":{"type":"invalid_request_error","message":"max_tokens: too big"}}"#;
    assert_eq!(retry_delay(400, None, bad, 0), None);
}

#[test]
fn an_explicit_zero_retry_after_means_now() {
    assert_eq!(retry_delay(429, Some(0), "{}", 0), Some(0));
    assert_eq!(
        retry_delay(529, None, "", 0),
        Some(1),
        "backoff starts at 1s"
    );
}

#[test]
fn again_counts_each_try_and_stops_when_the_retries_are_spent() {
    let mut attempt = 0;
    assert_eq!(
        again(529, Some(0), OVERLOADED, &mut attempt),
        Some(Duration::ZERO)
    );
    assert_eq!(
        again(529, None, OVERLOADED, &mut attempt),
        Some(Duration::from_secs(2))
    );
    assert_eq!(again(529, None, OVERLOADED, &mut attempt), None);
    assert_eq!(attempt, MAX_RETRIES);
    let mut fresh = 0;
    assert_eq!(again(400, None, "{}", &mut fresh), None);
    assert_eq!(fresh, 0, "a failure that stands is not counted");
}

#[test]
fn retry_after_reads_whole_seconds_and_ignores_the_rest() {
    let mut h = reqwest::header::HeaderMap::new();
    assert_eq!(retry_after(&h), None);
    h.insert(reqwest::header::RETRY_AFTER, " 3 ".parse().unwrap());
    assert_eq!(retry_after(&h), Some(3));
    // The HTTP-date form is not read; the backoff covers it.
    h.insert(
        reqwest::header::RETRY_AFTER,
        "Wed, 21 Oct 2026 07:28:00 GMT".parse().unwrap(),
    );
    assert_eq!(retry_after(&h), None);
}

/// A 2xx is a failure only when it carries a top-level `error`. The words
/// alone used to be enough, so a reply ABOUT rate limits was asked for again.
#[test]
fn a_2xx_reply_that_mentions_rate_limits_is_not_retried() {
    let reply = r#"{"choices":[{"message":{"role":"assistant","content":"add a rate limit to the endpoint"},"finish_reason":"stop","index":0}],"object":"chat.completion","usage":{"prompt_tokens":12,"completion_tokens":8,"total_tokens":20}}"#;
    assert_eq!(retry_delay(200, None, reply, 0), None);
    assert_eq!(retry_delay(200, None, "rate limit exceeded", 0), None);
    let nulled = r#"{"error":null,"choices":[],"note":"rate limit"}"#;
    assert_eq!(retry_delay(200, None, nulled, 0), None);
    // Not a 2xx: the words are still read wherever they are.
    assert_eq!(retry_delay(400, None, "rate limit exceeded", 0), Some(1));
}

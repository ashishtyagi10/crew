//! The OpenAI-compatible chat-completions HTTP layer shared by every
//! provider that speaks that shape (OpenRouter, Alibaba DashScope, …):
//! transient-error retry with Retry-After honouring, response parsing, and
//! SSE streaming.
use futures::StreamExt;
use serde::Deserialize;

use super::{ChunkFn, Completion, ProviderError};

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

/// One model's streamed request: same header/auth and transient-error retry
/// as [`request_with_retry`], plus SSE framing. `body` is the caller's base
/// request (model/messages/max_tokens) — `stream`/`stream_options` are added
/// here, not by the caller.
///
/// `stream_options.include_usage` is requested first; some OpenAI-compatible
/// endpoints reject the field with a 400, in which case this retries once
/// without it before falling back to the normal transient-retry loop (a 400
/// is never itself transient — see [`retry_delay`]).
///
/// `started` is flipped to `true` the moment the first visible [`SseItem::Delta`]
/// is forwarded to `on_chunk` — the caller uses it to tell "never got a
/// response worth showing" (safe to try the next model in a fallback chain)
/// apart from "already streamed visible text, then failed" (must NOT
/// silently retry elsewhere, since the caller has already forwarded partial
/// content through `on_chunk`). A 200 alone does not set it: OpenRouter's
/// wrapped-error shape (see [`retry_delay`]'s doc comment) is also a 200,
/// and must remain safe to retry/fall back on.
///
/// A 200 response is only ever treated as SSE when its `content-type` is not
/// `application/json` (belt-and-braces on top of [`consume_sse`]'s own
/// no-frames-ever-seen fallback below): OpenRouter's wrapped-error body is a
/// plain JSON object with no `data:` lines at all, so gating on content-type
/// avoids even attempting to parse it as a stream when the header is
/// available and says otherwise.
pub(super) async fn request_with_retry_streaming(
    client: &reqwest::Client,
    endpoint: &str,
    key: &str,
    body: &serde_json::Value,
    on_chunk: &ChunkFn,
    started: &std::sync::atomic::AtomicBool,
) -> Result<Completion, ProviderError> {
    let mut include_usage = true;
    let mut attempt = 0u32;
    loop {
        let mut req_body = body.clone();
        req_body["stream"] = serde_json::json!(true);
        if include_usage {
            req_body["stream_options"] = serde_json::json!({"include_usage": true});
        }
        let resp = client
            .post(endpoint)
            .header("authorization", format!("Bearer {key}"))
            .header("content-type", "application/json")
            .json(&req_body)
            .send()
            .await
            .map_err(|e| ProviderError::Http(e.to_string()))?;
        let status = resp.status().as_u16();
        let retry_after_hdr = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok());
        let is_json_ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|ct| ct.contains("application/json"));

        // Get a body to hand to the shared retry/parse tail below: either
        // return straight away with a real streamed Completion, or fall
        // through with the raw text of a non-stream (error) body — whether
        // that's a genuine non-2xx, a 200 whose content-type says JSON, or a
        // 200 that consume_sse determined never carried a single Done/Delta/
        // Usage frame (Critical-1: OpenRouter's wrapped-error shape must not
        // become a silent empty success).
        let text = if status == 200 && !is_json_ct {
            match consume_sse(resp, body, on_chunk, started).await? {
                SseOutcome::Completion(c) => return Ok(c),
                SseOutcome::NoContent(raw) => raw,
            }
        } else {
            resp.text()
                .await
                .map_err(|e| ProviderError::Http(e.to_string()))?
        };
        // Same status handling as the non-streaming path (including the
        // retry_delay integration), plus a one-shot fallback off
        // `stream_options` on a plain 400.
        if include_usage && status == 400 {
            include_usage = false;
            continue;
        }
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

/// The result of consuming an SSE body to completion: either a real
/// Completion (at least one [`SseItem::Delta`], [`SseItem::Usage`], or
/// [`SseItem::Done`] frame was seen), or — [`SseOutcome::NoContent`] — the
/// raw body when NONE of those ever arrived. The latter is OpenRouter's
/// wrapped-error shape (see [`retry_delay`]'s doc comment): a 200 whose body
/// is a plain JSON `error` object with no `data:` lines at all, which must
/// not be mistaken for an empty successful stream (Critical-1).
enum SseOutcome {
    Completion(Completion),
    NoContent(String),
}

/// Consume an OpenAI-compatible SSE body: bytes arrive in arbitrary chunks
/// (not aligned to line or even char boundaries), so a `carry: Vec<u8>`
/// buffer holds the trailing partial line across reads — split on `b'\n'`
/// (a UTF-8 continuation byte is never `0x0A`, so per-line splitting on raw
/// bytes is always safe) and only then lossily decoded one complete line at
/// a time, so a multi-byte codepoint straddling a chunk boundary is decoded
/// correctly (Important-3) rather than mangled by a per-chunk
/// `from_utf8_lossy`. Each complete line is classified by [`parse_sse_line`].
/// Deltas are forwarded to `on_chunk` and accumulated into the final text;
/// `[DONE]` stops the read. If the stream ends (EOF, not `[DONE]`) with one
/// final line still in `carry` (no trailing `\n` — often the usage frame),
/// it is parsed too (Important-2) rather than silently dropped. A transport
/// error partway through the stream is returned as-is — the caller must not
/// synthesize a partial success.
///
/// `req_body` (the pre-`stream` request JSON) only backs the chars/4 token
/// estimate used when no `usage` frame ever arrives (e.g. the endpoint
/// doesn't honor `stream_options.include_usage`), mirroring the chars/4
/// heuristic this streaming feature uses elsewhere for token estimation.
///
/// `started` is flipped to `true` on the first [`SseItem::Delta`] forwarded
/// to `on_chunk` (see [`request_with_retry_streaming`]'s doc comment).
async fn consume_sse(
    resp: reqwest::Response,
    req_body: &serde_json::Value,
    on_chunk: &ChunkFn,
    started: &std::sync::atomic::AtomicBool,
) -> Result<SseOutcome, ProviderError> {
    let mut stream = resp.bytes_stream();
    let mut carry: Vec<u8> = Vec::new();
    let mut raw_bytes: Vec<u8> = Vec::new();
    let mut text = String::new();
    let mut usage: Option<(u64, u64)> = None;
    // Any Done/Delta/Usage frame ever seen — distinguishes a genuine (if
    // empty) stream from a non-SSE error body (Critical-1).
    let mut any_frame = false;
    'read: while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| ProviderError::Http(e.to_string()))?;
        raw_bytes.extend_from_slice(&bytes);
        carry.extend_from_slice(&bytes);
        while let Some(pos) = carry.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = carry.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line_bytes[..line_bytes.len() - 1]);
            let line = line.trim_end_matches('\r');
            if apply_sse_line(
                line,
                on_chunk,
                started,
                &mut any_frame,
                &mut text,
                &mut usage,
            ) {
                break 'read;
            }
        }
    }
    // Leftover carry at EOF: a final line with no trailing `\n` (Important-2).
    if !carry.is_empty() {
        let line = String::from_utf8_lossy(&carry);
        let line = line.trim_end_matches('\r');
        apply_sse_line(
            line,
            on_chunk,
            started,
            &mut any_frame,
            &mut text,
            &mut usage,
        );
    }
    if !any_frame {
        return Ok(SseOutcome::NoContent(
            String::from_utf8_lossy(&raw_bytes).into_owned(),
        ));
    }
    let (input_tokens, output_tokens) = match usage {
        Some((i, o)) => (i.min(u32::MAX as u64) as u32, o.min(u32::MAX as u64) as u32),
        None => (
            estimate_input_tokens(req_body),
            estimate_output_tokens(&text),
        ),
    };
    Ok(SseOutcome::Completion(Completion {
        text,
        input_tokens,
        output_tokens,
    }))
}

/// Classify and apply one complete SSE line to the accumulating stream
/// state (shared by the main read loop and the leftover-carry pass in
/// [`consume_sse`]). Returns `true` if this was the `[DONE]` frame (the
/// caller should stop reading).
fn apply_sse_line(
    line: &str,
    on_chunk: &ChunkFn,
    started: &std::sync::atomic::AtomicBool,
    any_frame: &mut bool,
    text: &mut String,
    usage: &mut Option<(u64, u64)>,
) -> bool {
    match parse_sse_line(line) {
        SseItem::Delta(s) => {
            // Unconditional: a usage frame arriving BEFORE the first delta
            // (legal for any OpenAI-shaped backend) sets `any_frame`, and a
            // guarded store would then never flip `started` — letting a
            // mid-stream failure retry into another model and splice text,
            // the exact thing this flag prevents. The store is idempotent.
            started.store(true, std::sync::atomic::Ordering::SeqCst);
            *any_frame = true;
            text.push_str(&s);
            on_chunk(&s);
            false
        }
        SseItem::Usage(i, o) => {
            *usage = Some((i, o));
            *any_frame = true;
            false
        }
        SseItem::Done => {
            *any_frame = true;
            true
        }
        SseItem::Skip => false,
    }
}

/// ~4 chars/token fallback estimate for streamed output text (see
/// [`consume_sse`]). Floored at 1 token when the text is non-empty — chars/4
/// truncates to 0 for anything under 4 chars, which would otherwise
/// misreport a real (if tiny) reply as having produced no output tokens.
fn estimate_output_tokens(s: &str) -> u32 {
    let n = chars_to_tokens(s.chars().count());
    if s.is_empty() {
        n
    } else {
        n.max(1)
    }
}

/// Same chars/4 estimate as [`estimate_output_tokens`] (minus its non-empty
/// floor), applied to the request's message contents (fallback input-token
/// count when no `usage` frame arrives). Counts `chars()`, not bytes: an
/// earlier version counted `str::len()` (bytes) here while the output side
/// counted chars, a 3x divergence on multi-byte text such as CJK.
fn estimate_input_tokens(req_body: &serde_json::Value) -> u32 {
    let chars: usize = req_body["messages"]
        .as_array()
        .map(|msgs| {
            msgs.iter()
                .filter_map(|m| m["content"].as_str())
                .map(|s| s.chars().count())
                .sum()
        })
        .unwrap_or(0);
    chars_to_tokens(chars)
}

fn chars_to_tokens(chars: usize) -> u32 {
    ((chars as u64) / 4).min(u32::MAX as u64) as u32
}

/// One parsed SSE line from an OpenAI-compatible streaming response.
pub(crate) enum SseItem {
    Delta(String),
    Usage(u64, u64),
    Done,
    Skip,
}

/// Pure classifier for one SSE line: `data: [DONE]`, a delta frame, a
/// usage frame, or noise (keep-alives, blanks, junk) → Skip. Never errors:
/// a malformed frame is ignored and the stream carries on.
pub(crate) fn parse_sse_line(line: &str) -> SseItem {
    let Some(data) = line.strip_prefix("data:").map(str::trim) else {
        return SseItem::Skip;
    };
    if data == "[DONE]" {
        return SseItem::Done;
    }
    let Ok(v) = serde_json::from_str::<serde_json::Value>(data) else {
        return SseItem::Skip;
    };
    if let Some(s) = v["choices"][0]["delta"]["content"].as_str() {
        if !s.is_empty() {
            return SseItem::Delta(s.to_string());
        }
    }
    if let Some(u) = v.get("usage").filter(|u| !u.is_null()) {
        let i = u["prompt_tokens"].as_u64().unwrap_or(0);
        let o = u["completion_tokens"].as_u64().unwrap_or(0);
        if i > 0 || o > 0 {
            return SseItem::Usage(i, o);
        }
    }
    SseItem::Skip
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

//! The OpenAI-compatible chat-completions HTTP layer shared by every
//! provider that speaks that shape (OpenRouter, Alibaba DashScope, …):
//! transient-error retry (the rule itself is [`super::retry`]'s, shared with
//! Anthropic), response parsing, and SSE streaming.
use futures::StreamExt;
use serde::Deserialize;

use super::retry::{again, retry_after};
use super::ssecalls::{frags, CallAsm, Frag};
use super::status::settle;
use super::thinktags::{Piece, ThinkTags};
use super::utf8carry::Utf8Carry;
use super::wire::wire_error;
use super::{served, stopreason, thinking, Chunk, ChunkFn, Completion, ProviderError};

/// One model's request with transient-error retry (see [`super::retry`]).
///
/// A body carrying a reasoning opt-in (`thinking::opt_in`) that comes back
/// 400 is retried ONCE without it: the field is the newest thing in the
/// request and the one an endpoint is likeliest not to know.
pub(super) async fn request_with_retry(
    client: &reqwest::Client,
    endpoint: &str,
    key: &str,
    body: &serde_json::Value,
) -> Result<Completion, ProviderError> {
    let mut attempt = 0u32;
    let mut body = body.clone();
    loop {
        let resp = client
            .post(endpoint)
            .header("authorization", format!("Bearer {key}"))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Http(wire_error(&e, endpoint)))?;
        let status = resp.status().as_u16();
        let retry_after_hdr = retry_after(resp.headers());
        let text = resp
            .text()
            .await
            .map_err(|e| ProviderError::Http(wire_error(&e, endpoint)))?;
        if status == 400 && thinking::strip(&mut body) {
            continue;
        }
        if let Some(wait) = again(status, retry_after_hdr, &text, &mut attempt) {
            tokio::time::sleep(wait).await;
            continue;
        }
        return settle(status, endpoint, &text, parse_response);
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
/// is never itself transient — see `retry::retry_delay`). A reasoning opt-in in
/// `body` (`thinking::opt_in`) is dropped the same way, and FIRST: it is the
/// newer field and the likelier stranger.
///
/// `started` is flipped to `true` the moment the first visible [`SseItem::Delta`]
/// is forwarded to `on_chunk` — the caller uses it to tell "never got a
/// response worth showing" (safe to try the next model in a fallback chain)
/// apart from "already streamed visible text, then failed" (must NOT
/// silently retry elsewhere, since the caller has already forwarded partial
/// content through `on_chunk`). A 200 alone does not set it: OpenRouter's
/// wrapped-error shape (see `retry::retry_delay`'s doc comment) is also a 200,
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
    let mut body = body.clone();
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
            .map_err(|e| ProviderError::Http(wire_error(&e, endpoint)))?;
        let status = resp.status().as_u16();
        let retry_after_hdr = retry_after(resp.headers());
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
            match consume_sse(resp, endpoint, &body, on_chunk, started).await? {
                SseOutcome::Completion(c) => return Ok(c),
                SseOutcome::NoContent(raw) => raw,
            }
        } else {
            resp.text()
                .await
                .map_err(|e| ProviderError::Http(wire_error(&e, endpoint)))?
        };
        // Same status handling as the non-streaming path (including the
        // shared retry rule), plus a one-shot fallback off the
        // reasoning opt-in, then off `stream_options`, on a plain 400.
        if status == 400 && thinking::strip(&mut body) {
            continue;
        }
        if include_usage && status == 400 {
            include_usage = false;
            continue;
        }
        if let Some(wait) = again(status, retry_after_hdr, &text, &mut attempt) {
            tokio::time::sleep(wait).await;
            continue;
        }
        return settle(status, endpoint, &text, parse_response);
    }
}

/// The result of consuming an SSE body to completion: either a real
/// Completion (at least one [`SseItem::Delta`], [`SseItem::Usage`], or
/// [`SseItem::Done`] frame was seen), or — [`SseOutcome::NoContent`] — the
/// raw body when NONE of those ever arrived. The latter is OpenRouter's
/// wrapped-error shape (see `retry::retry_delay`'s doc comment): a 200 whose body
/// is a plain JSON `error` object with no `data:` lines at all, which must
/// not be mistaken for an empty successful stream (Critical-1).
enum SseOutcome {
    Completion(Completion),
    NoContent(String),
}

/// Consume an OpenAI-compatible SSE body: bytes arrive in arbitrary chunks
/// (not aligned to line or even char boundaries), so they are decoded
/// through a [`Utf8Carry`] — a multi-byte codepoint straddling a chunk
/// boundary is decoded whole (Important-3) rather than mangled by a
/// per-chunk `from_utf8_lossy` — and a `carry` string holds the trailing
/// partial line across reads. Each complete line is classified by
/// [`parse_sse_frame`].
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
    endpoint: &str,
    req_body: &serde_json::Value,
    on_chunk: &ChunkFn,
    started: &std::sync::atomic::AtomicBool,
) -> Result<SseOutcome, ProviderError> {
    let mut stream = resp.bytes_stream();
    let mut utf8 = Utf8Carry::default();
    let mut carry = String::new();
    let mut raw_bytes: Vec<u8> = Vec::new();
    let mut st = SseState::default();
    'read: while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| ProviderError::Http(wire_error(&e, endpoint)))?;
        raw_bytes.extend_from_slice(&bytes);
        carry.push_str(&utf8.push(&bytes));
        while let Some(pos) = carry.find('\n') {
            let line: String = carry.drain(..=pos).collect();
            if apply_sse_line(
                line.trim_end_matches(['\n', '\r']),
                on_chunk,
                started,
                &mut st,
            ) {
                break 'read;
            }
        }
    }
    // Leftover carry at EOF: a final line with no trailing `\n` (Important-2).
    carry.push_str(&utf8.finish());
    if !carry.is_empty() {
        apply_sse_line(carry.trim_end_matches('\r'), on_chunk, started, &mut st);
    }
    if !st.any_frame {
        return Ok(SseOutcome::NoContent(
            String::from_utf8_lossy(&raw_bytes).into_owned(),
        ));
    }
    // A `<thi` held back at the very end was never a tag.
    for piece in st.tags.finish() {
        st.route(piece, on_chunk, started);
    }
    let (input_tokens, output_tokens, cost_microusd) = match st.usage {
        Some((i, o, cost)) => (
            i.min(u32::MAX as u64) as u32,
            o.min(u32::MAX as u64) as u32,
            cost,
        ),
        None => (
            estimate_input_tokens(req_body),
            estimate_output_tokens(&st.text),
            0,
        ),
    };
    Ok(SseOutcome::Completion(Completion {
        text: st.text,
        thought: st.thought.trim().to_string(),
        model: st.model,
        input_tokens,
        cached_input_tokens: st.cached.0,
        cache_write_tokens: st.cached.1,
        output_tokens,
        cost_microusd,
        calls: st.calls.finish(),
        truncated: st.truncated,
    }))
}

/// Everything a stream accumulates between its first frame and `[DONE]`.
#[derive(Default)]
struct SseState {
    text: String,
    thought: String,
    /// `<think>` tags inside `delta.content` (see [`ThinkTags`]).
    tags: ThinkTags,
    calls: CallAsm,
    usage: Option<(u64, u64, u64)>,
    /// `(read, written)` prompt-cache tokens, and the answering model, both
    /// off the usage frame.
    cached: (u32, u32),
    model: String,
    /// The last `finish_reason` said the token ceiling ended it.
    truncated: bool,
    /// Any Done/Delta/Thought/Calls/Usage frame ever seen — distinguishes a
    /// genuine (if empty) stream from a non-SSE error body (Critical-1).
    any_frame: bool,
}

impl SseState {
    /// Keep one routed run and forward it.
    fn route(&mut self, piece: Piece, on_chunk: &ChunkFn, started: &std::sync::atomic::AtomicBool) {
        match piece {
            Piece::Text(s) => {
                // Unconditional: a usage frame arriving BEFORE the first
                // delta (legal for any OpenAI-shaped backend) sets
                // `any_frame`, and a guarded store would then never flip
                // `started` — letting a mid-stream failure retry into
                // another model and splice text, the exact thing this flag
                // prevents. The store is idempotent. Reasoning does NOT set
                // it: a fallback model splicing its own thinking under a
                // dead one's is harmless, while the reply must stay whole.
                started.store(true, std::sync::atomic::Ordering::SeqCst);
                self.text.push_str(&s);
                on_chunk(Chunk::Text(&s));
            }
            Piece::Thought(s) => {
                self.thought.push_str(&s);
                on_chunk(Chunk::Thought(&s));
            }
        }
    }
}

/// Classify and apply one complete SSE line to the accumulating stream
/// state (shared by the main read loop and the leftover-carry pass in
/// [`consume_sse`]). Returns `true` if this was the `[DONE]` frame (the
/// caller should stop reading).
fn apply_sse_line(
    line: &str,
    on_chunk: &ChunkFn,
    started: &std::sync::atomic::AtomicBool,
    st: &mut SseState,
) -> bool {
    let mut done = false;
    for item in parse_sse_frame(line) {
        st.any_frame = true;
        match item {
            SseItem::Delta(s) => {
                for piece in st.tags.feed(&s) {
                    st.route(piece, on_chunk, started);
                }
            }
            SseItem::Thought(s) => st.route(Piece::Thought(s), on_chunk, started),
            SseItem::Calls(fs) => fs.into_iter().for_each(|f| st.calls.push(f)),
            SseItem::Usage(i, o, cost) => st.usage = Some((i, o, cost)),
            SseItem::Cached(read, written) => st.cached = (read, written),
            SseItem::Model(m) => st.model = m,
            SseItem::Stop(why) => st.truncated = stopreason::openai(Some(&why)),
            SseItem::Done => done = true,
        }
    }
    done
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

/// One parsed item of an OpenAI-compatible SSE frame.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SseItem {
    /// Reply text (`delta.content`).
    Delta(String),
    /// Reasoning: DashScope/vLLM/NIM `delta.reasoning_content`, OpenRouter
    /// `delta.reasoning` or `delta.reasoning_details[].text|summary`.
    Thought(String),
    /// Tool-call fragments (`delta.tool_calls`), see [`CallAsm`].
    Calls(Vec<Frag>),
    /// `finish_reason`, on the frame that ends the choice. The last one
    /// wins, and only `"length"` matters (`stopreason`).
    Stop(String),
    Usage(u64, u64, u64),
    /// `(read, written)` prompt-cache tokens on the usage frame
    /// (`served::openai_cache`), when there were any.
    Cached(u32, u32),
    /// The model the usage frame names — the one that answered.
    Model(String),
    Done,
}

/// The reasoning text a delta carries, under whichever of the three names
/// this endpoint uses. `reasoning_details` is OpenRouter's structured form —
/// `text` for models that show their reasoning, `summary` for ones that only
/// summarise it; encrypted-only entries have neither and yield nothing.
fn reasoning_of(delta: &serde_json::Value) -> String {
    let mut out = String::new();
    for key in ["reasoning_content", "reasoning"] {
        if let Some(s) = delta[key].as_str() {
            out.push_str(s);
        }
    }
    if let Some(details) = delta["reasoning_details"].as_array() {
        for d in details {
            if let Some(s) = d["text"].as_str().or_else(|| d["summary"].as_str()) {
                out.push_str(s);
            }
        }
    }
    out
}

/// Pure classifier for one SSE line: everything the frame carries, in the
/// order it should apply — reasoning, then text, then tool-call fragments,
/// then why it stopped, then usage — or nothing for noise (keep-alives, blanks, junk). One frame
/// CAN carry several: some endpoints put the usage on the last content
/// frame, and a first-item-only read dropped it. Never errors: a malformed
/// frame is ignored and the stream carries on.
pub(crate) fn parse_sse_frame(line: &str) -> Vec<SseItem> {
    let Some(data) = line.strip_prefix("data:").map(str::trim) else {
        return Vec::new();
    };
    if data == "[DONE]" {
        return vec![SseItem::Done];
    }
    let Ok(v) = serde_json::from_str::<serde_json::Value>(data) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let delta = &v["choices"][0]["delta"];
    let thought = reasoning_of(delta);
    if !thought.is_empty() {
        out.push(SseItem::Thought(thought));
    }
    if let Some(s) = delta["content"].as_str().filter(|s| !s.is_empty()) {
        out.push(SseItem::Delta(s.to_string()));
    }
    let fs = frags(delta);
    if !fs.is_empty() {
        out.push(SseItem::Calls(fs));
    }
    if let Some(why) = v["choices"][0]["finish_reason"].as_str() {
        out.push(SseItem::Stop(why.to_string()));
    }
    if let Some(u) = v.get("usage").filter(|u| !u.is_null()) {
        let i = u["prompt_tokens"].as_u64().unwrap_or(0);
        let o = u["completion_tokens"].as_u64().unwrap_or(0);
        let cost = (u["cost"].as_f64().unwrap_or(0.0) * 1_000_000.0) as u64;
        if i > 0 || o > 0 {
            out.push(SseItem::Usage(i, o, cost));
            let (read, written) = served::openai_cache(&u["prompt_tokens_details"]);
            if read > 0 || written > 0 {
                out.push(SseItem::Cached(read, written));
            }
            let model = served::model_of(&v);
            if !model.is_empty() {
                out.push(SseItem::Model(model));
            }
        }
    }
    out
}

#[derive(Deserialize)]
struct FnCall {
    #[serde(default)]
    name: String,
    /// A JSON *string*, not an object — the OpenAI shape sends arguments
    /// serialised, and models sometimes send `""` for a no-argument call.
    #[serde(default)]
    arguments: String,
}

#[derive(Deserialize)]
struct RawToolCall {
    #[serde(default)]
    id: String,
    #[serde(default)]
    function: Option<FnCall>,
}

#[derive(Deserialize, Default)]
struct Msg {
    /// `Option`, not a defaulted `String`: a reply that ONLY calls tools sends
    /// an explicit `"content": null`, and `#[serde(default)]` does not cover
    /// an explicit null — it would fail the whole parse at exactly the moment
    /// tool use started working.
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<RawToolCall>,
    /// The model's reasoning, under the two names the non-streamed shape
    /// uses (DashScope/vLLM, then OpenRouter). Both `Option`: an explicit
    /// null is common here too.
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    reasoning: Option<String>,
}

#[derive(Deserialize)]
struct Choice {
    #[serde(default)]
    message: Msg,
    /// Why generation stopped — `"length"` is the token ceiling.
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    /// OpenRouter-only: exact request cost in USD when the request asked
    /// for it (`usage: {include: true}`). Absent everywhere else; `Option`
    /// so an explicit `"cost": null` decodes instead of failing the parse.
    #[serde(default)]
    cost: Option<f64>,
    /// `cached_tokens` and friends (`served::openai_cache`), read loosely:
    /// its shape varies by host and a surprise in it must not fail the parse.
    #[serde(default)]
    prompt_tokens_details: serde_json::Value,
}

#[derive(Deserialize)]
struct ApiResp {
    #[serde(default)]
    choices: Vec<Choice>,
    usage: Option<Usage>,
    /// The model that answered (`served`); `Option` for an explicit null.
    #[serde(default)]
    model: Option<String>,
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
    let msg = r.choices.first().map(|c| &c.message);
    let truncated = stopreason::openai(r.choices.first().and_then(|c| c.finish_reason.as_deref()));
    // `<think>` tags in the body route to reasoning here exactly as they do
    // in a stream (`ThinkTags`), so the two paths agree on what the reply IS.
    let (text, mut thought) =
        ThinkTags::split(msg.and_then(|m| m.content.as_deref()).unwrap_or(""));
    if let Some(r) = msg.and_then(|m| m.reasoning_content.as_deref().or(m.reasoning.as_deref())) {
        thought.insert_str(0, r);
    }
    let calls: Vec<super::ToolInvocation> = msg
        .map(|m| {
            m.tool_calls
                .iter()
                .filter_map(|tc| {
                    let f = tc.function.as_ref()?;
                    Some(super::toolargs::invocation(
                        tc.id.clone(),
                        f.name.clone(),
                        &f.arguments,
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let usage = r
        .usage
        .ok_or_else(|| ProviderError::Decode("missing usage".into()))?;
    let (cached_input_tokens, cache_write_tokens) =
        served::openai_cache(&usage.prompt_tokens_details);
    Ok(Completion {
        text,
        thought: thought.trim().to_string(),
        model: r.model.unwrap_or_default().trim().to_string(),
        input_tokens: usage.prompt_tokens,
        cached_input_tokens,
        cache_write_tokens,
        output_tokens: usage.completion_tokens,
        cost_microusd: (usage.cost.unwrap_or(0.0) * 1_000_000.0) as u64,
        calls,
        truncated,
    })
}

#[cfg(test)]
#[path = "openai_http_tests.rs"]
mod tests;

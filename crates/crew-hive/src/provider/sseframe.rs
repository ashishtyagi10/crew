//! One line of an OpenAI-compatible stream, read: everything a `data:` frame
//! carries, as [`SseItem`]s.
//!
//! Pure over one line, so the framing is tested without a socket. What a
//! whole body adds up to — and what it means when the body stops before the
//! stream said it was over — is `openai_http::consume_sse`'s to decide.
use super::served;
use super::ssecalls::{frags, Frag};

/// One parsed item of an OpenAI-compatible SSE frame.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SseItem {
    /// Reply text (`delta.content`).
    Delta(String),
    /// Reasoning: DashScope/vLLM/NIM `delta.reasoning_content`, OpenRouter
    /// `delta.reasoning` or `delta.reasoning_details[].text|summary`.
    Thought(String),
    /// Tool-call fragments (`delta.tool_calls`), see `ssecalls::CallAsm`.
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

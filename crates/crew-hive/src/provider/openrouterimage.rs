//! Images in an OpenAI-wire request: `{"type":"image_url","image_url":{"url":
//! "data:<type>;base64,..."}}` parts of a user message. With no image every
//! function returns what the request always carried.
//!
//! Chat Completions `tool` messages are text-only, so a tool result's pictures
//! cannot ride in the result. They go in ONE user message right after the
//! batch's last `tool` message (never between two, which the API rejects: every
//! `tool_call_id` must be answered before anything else), each labelled with
//! the call it came from. Skipping them would make `sys:read_file` of a
//! screenshot useless on this wire; a following user message costs one extra
//! message and keeps earlier turns untouched.
use serde_json::{json, Value};

use super::image::Image;
use super::imagevision::{split, with_notes};

fn part(i: &Image) -> Value {
    json!({"type": "image_url", "image_url": {"url": i.data_url()}})
}

/// The opening user message's `content`: the plain prompt, or a parts array
/// (text, then each picture) when `model` is given any.
pub(super) fn user_content(model: &str, prompt: &str, images: &[Image]) -> Value {
    if images.is_empty() {
        return json!(prompt);
    }
    let (shown, notes) = split(model, images);
    let text = with_notes(prompt, &notes);
    if shown.is_empty() {
        return json!(text);
    }
    let mut parts = vec![json!({"type": "text", "text": text})];
    parts.extend(shown.into_iter().map(part));
    json!(parts)
}

/// A tool result's text with a note for each picture `model` cannot be given.
pub(super) fn tool_text(model: &str, text: &str, images: &[Image]) -> String {
    with_notes(text, &split(model, images).1)
}

/// The user message that carries a batch's pictures, or `None` when `model`
/// is given none of them.
pub(super) fn follow_up(model: &str, results: &[super::ToolOutcome]) -> Option<Value> {
    let mut parts = Vec::new();
    for r in results {
        for i in split(model, &r.images).0 {
            parts.push(
                json!({"type": "text", "text": format!("Image from {}: {}", r.name, i.name)}),
            );
            parts.push(part(i));
        }
    }
    (!parts.is_empty()).then(|| json!({"role": "user", "content": parts}))
}

/// Whether `req` carries a picture anywhere, so a fallback model must be
/// rendered its own request.
pub(super) fn carries(req: &super::CompletionRequest) -> bool {
    !req.images.is_empty()
        || req.turns.iter().any(|t| {
            matches!(t, super::Turn::ToolResults(rs) if rs.iter().any(|r| !r.images.is_empty()))
        })
}

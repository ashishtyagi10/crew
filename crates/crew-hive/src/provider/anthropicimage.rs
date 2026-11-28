//! Images in an Anthropic request: `{"type":"image","source":{"type":"base64",
//! ...}}` blocks, in a user message and inside a `tool_result`'s content array.
//! Kept out of `anthropic.rs` (far over the line cap). With no image both
//! functions return the plain string the request always carried, so a request
//! without pictures is byte-identical to what it was.
use serde_json::{json, Value};

use super::image::Image;
use super::imagevision::{split, with_notes};

fn block(i: &Image) -> Value {
    json!({"type": "image", "source": {"type": "base64", "media_type": i.media_type, "data": i.data()}})
}

/// The opening user message: its pictures (Anthropic reads best with the
/// image before the question), then the text, which carries a note for any
/// picture `model` cannot be given.
pub(crate) fn user_content(model: &str, prompt: &str, images: &[Image]) -> Value {
    if images.is_empty() {
        return json!(prompt);
    }
    let (shown, notes) = split(model, images);
    let text = with_notes(prompt, &notes);
    if shown.is_empty() {
        return json!(text);
    }
    let mut parts: Vec<Value> = shown.into_iter().map(block).collect();
    parts.push(json!({"type": "text", "text": text}));
    json!(parts)
}

/// A `tool_result`'s `content`: the text alone as a string, or, with pictures,
/// an array of the text block then the images.
pub(crate) fn result_content(model: &str, content: &str, images: &[Image]) -> Value {
    if images.is_empty() {
        return json!(content);
    }
    let (shown, notes) = split(model, images);
    let text = with_notes(content, &notes);
    if shown.is_empty() {
        return json!(text);
    }
    let mut parts = vec![json!({"type": "text", "text": text})];
    parts.extend(shown.into_iter().map(block));
    json!(parts)
}

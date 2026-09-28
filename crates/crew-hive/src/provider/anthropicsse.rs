//! Anthropic's streamed Messages events, folded into a [`Completion`].
//!
//! A reply served over the Anthropic API — a key, or a signed-in `ant`
//! profile — used to arrive whole: the provider had no streamed path, so the
//! pane sat on "thinking" for the entire generation and then printed the
//! answer at once, while every OpenAI-compatible host typed it out live. This
//! is the event reader for `"stream": true`: `message_start` carries the
//! input usage, `content_block_delta` the `text_delta` / `thinking_delta`
//! fragments, `message_delta` the output usage, `error` an in-stream failure.
//! Pure over the bytes it is fed, so the framing is tested without a socket.
use super::{Chunk, ChunkFn, Completion, ProviderError};

/// Streaming state: the unfinished line, and the reply so far.
#[derive(Default)]
pub(crate) struct Fold {
    pending: String,
    pub(crate) out: Completion,
    /// An `error` event arrived mid-stream.
    pub(crate) failed: Option<String>,
    /// Any event parsed at all. A server that ignores `"stream": true` (some
    /// gateways, a test stub) answers with the ordinary JSON body, which has
    /// no `data:` lines — [`Self::finish`] reads that body as a whole reply.
    seen: bool,
    /// The bytes, while nothing has parsed as an event.
    raw: String,
}

impl Fold {
    /// Feed raw bytes as they arrive, forwarding each fragment to `on_chunk`.
    pub(crate) fn feed(&mut self, bytes: &str, on_chunk: &ChunkFn) {
        if !self.seen {
            self.raw.push_str(bytes);
        }
        self.pending.push_str(bytes);
        while let Some(i) = self.pending.find('\n') {
            let line: String = self.pending.drain(..=i).collect();
            if let Some(data) = line.trim_end().strip_prefix("data:") {
                self.event(data.trim(), on_chunk);
            }
        }
    }

    fn event(&mut self, data: &str, on_chunk: &ChunkFn) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(data) else {
            return;
        };
        if !self.seen {
            self.seen = true;
            self.raw = String::new();
        }
        match v["type"].as_str().unwrap_or("") {
            "message_start" => {
                let u = &v["message"]["usage"];
                self.out.input_tokens = u["input_tokens"].as_u64().unwrap_or(0) as u32;
            }
            "content_block_delta" => {
                let d = &v["delta"];
                match d["type"].as_str().unwrap_or("") {
                    "text_delta" => {
                        let t = d["text"].as_str().unwrap_or("");
                        self.out.text.push_str(t);
                        on_chunk(Chunk::Text(t));
                    }
                    "thinking_delta" => {
                        let t = d["thinking"].as_str().unwrap_or("");
                        self.out.thought.push_str(t);
                        on_chunk(Chunk::Thought(t));
                    }
                    _ => {}
                }
            }
            "message_delta" => {
                if let Some(n) = v["usage"]["output_tokens"].as_u64() {
                    self.out.output_tokens = n as u32;
                }
            }
            "error" => self.failed = Some(v["error"].to_string()),
            _ => {}
        }
    }

    /// The reply, or the stream's own error — or, when the server sent a
    /// whole JSON reply instead of events, that reply, forwarded as one chunk.
    pub(crate) fn finish(self, on_chunk: &ChunkFn) -> Result<Completion, ProviderError> {
        if !self.seen {
            let c = super::AnthropicProvider::parse_response(&self.raw)?;
            if !c.text.is_empty() {
                on_chunk(Chunk::Text(&c.text));
            }
            return Ok(c);
        }
        match self.failed {
            Some(e) => Err(ProviderError::Api(e)),
            None => Ok(self.out),
        }
    }
}

#[cfg(test)]
#[path = "anthropicsse_tests.rs"]
mod tests;

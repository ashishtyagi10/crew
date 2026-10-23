//! Anthropic's streamed Messages events, folded into a [`Completion`].
//!
//! A reply served over the Anthropic API — a key, or a signed-in `ant`
//! profile — used to arrive whole: the provider had no streamed path, so the
//! pane sat on "thinking" for the entire generation and then printed the
//! answer at once, while every OpenAI-compatible host typed it out live. This
//! is the event reader for `"stream": true`: `message_start` carries the
//! input usage (cache reads and writes too) and the answering model,
//! `content_block_delta` the `text_delta` / `thinking_delta`
//! fragments, `message_delta` the output usage and why it stopped, `error`
//! an in-stream failure.
//! Pure over the bytes it is fed, so the framing is tested without a socket.
use super::{Chunk, ChunkFn, Completion, ProviderError};

/// Streaming state: the unfinished line, and the reply so far.
#[derive(Default)]
pub(crate) struct Fold {
    pending: String,
    pub(crate) out: Completion,
    /// An `error` event arrived mid-stream: the whole event, which is the
    /// same `{"type":"error","error":{…}}` envelope a refused request's body
    /// is, so the retry rule (a 2xx is an error only when it has a top-level
    /// `error`) and `api_message` read both alike.
    pub(crate) failed: Option<String>,
    /// Any event parsed at all. A server that ignores `"stream": true` (some
    /// gateways, a test stub) answers with the ordinary JSON body, which has
    /// no `data:` lines — [`Self::finish`] reads that body as a whole reply.
    seen: bool,
    /// The bytes, while nothing has parsed as an event.
    raw: String,
    /// The stream said it was over: `message_stop`, or a `message_delta`
    /// with its `stop_reason`. A body that closes cleanly without either was
    /// cut ([`Self::finish`]).
    ended: bool,
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
                self.out.input_tokens = super::served::anthropic_input(u);
                let (read, written) = super::served::anthropic_cache(u);
                self.out.cached_input_tokens = read;
                self.out.cache_write_tokens = written;
                self.out.model = super::served::model_of(&v["message"]);
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
                if let Some(why) = v["delta"]["stop_reason"].as_str() {
                    self.out.truncated = super::stopreason::anthropic(Some(why));
                    self.ended = true;
                }
            }
            "message_stop" => self.ended = true,
            "error" => self.failed = Some(v.to_string()),
            _ => {}
        }
    }

    /// The reply, or the stream's own error — or, when the server sent a
    /// whole JSON reply instead of events, that reply, forwarded as one chunk.
    /// Words cut off by a clean close from `endpoint` fail as a broken stream
    /// does: as a reply they stood as the whole answer, ending mid-word, and
    /// only a failed stream is picked up where it stopped.
    pub(crate) fn finish(
        self,
        endpoint: &str,
        on_chunk: &ChunkFn,
    ) -> Result<Completion, ProviderError> {
        if !self.seen {
            let c = super::AnthropicProvider::parse_response(&self.raw)?;
            if !c.text.is_empty() {
                on_chunk(Chunk::Text(&c.text));
            }
            return Ok(c);
        }
        match self.failed {
            Some(e) => Err(ProviderError::Api(e)),
            None if !self.ended && !self.out.text.trim().is_empty() => {
                Err(ProviderError::Http(format!(
                    "{} closed the stream before the reply ended (no message_stop, no stop_reason)",
                    super::wire::host_of(endpoint)
                )))
            }
            None => Ok(self.out),
        }
    }
}

#[cfg(test)]
#[path = "anthropicsse_tests.rs"]
mod tests;

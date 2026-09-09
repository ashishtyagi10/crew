use std::future::Future;
use std::pin::Pin;

use super::thinktags::ThinkTags;
use super::{Chunk, ChunkFn, Completion, CompletionRequest, Provider, ProviderError};

/// Deterministic provider for headless tests: returns `reply` and counts tokens
/// by whitespace.
///
/// A reply that opens with `<think>…</think>` reasons first: the fenced text
/// is the completion's `thought` (streamed as one `Chunk::Thought` before any
/// text) and the rest is the reply — the same split the real OpenAI-shaped
/// path makes, so a test of the thinking pipeline needs no second mock.
pub struct MockProvider {
    pub reply: String,
}

impl Provider for MockProvider {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let (text, thought) = ThinkTags::split(&self.reply);
        Box::pin(async move {
            Ok(Completion {
                input_tokens: req.prompt.split_whitespace().count() as u32,
                output_tokens: text.split_whitespace().count() as u32,
                cost_microusd: 0,
                text,
                thought,
                ..Default::default()
            })
        })
    }

    /// Splits the reply text into 3 roughly-equal word groups (fewer if the
    /// reply is shorter) and calls `on_chunk` per group, in order — after the
    /// thought, if any — before resolving with the same `Completion`
    /// `complete` builds.
    fn complete_streaming(
        &self,
        req: CompletionRequest,
        on_chunk: ChunkFn,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let (reply, thought) = ThinkTags::split(&self.reply);
        let fut = self.complete(req);
        Box::pin(async move {
            if !thought.is_empty() {
                on_chunk(Chunk::Thought(&thought));
            }
            let words: Vec<&str> = reply.split_whitespace().collect();
            let per = words.len().div_ceil(3).max(1);
            let mut sent = 0;
            for group in words.chunks(per) {
                // Reconstruct with the separating spaces so chunks concat to the reply.
                let mut s = group.join(" ");
                sent += group.len();
                if sent < words.len() {
                    s.push(' ');
                }
                on_chunk(Chunk::Text(&s));
            }
            fut.await
        })
    }
}

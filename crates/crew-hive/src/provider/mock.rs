use std::future::Future;
use std::pin::Pin;

use super::{ChunkFn, Completion, CompletionRequest, Provider, ProviderError};

/// Deterministic provider for headless tests: returns `reply` and counts tokens
/// by whitespace.
pub struct MockProvider {
    pub reply: String,
}

impl Provider for MockProvider {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let reply = self.reply.clone();
        Box::pin(async move {
            Ok(Completion {
                text: reply.clone(),
                input_tokens: req.prompt.split_whitespace().count() as u32,
                output_tokens: reply.split_whitespace().count() as u32,
            })
        })
    }

    /// Splits `reply` into 3 roughly-equal word groups (fewer if the reply
    /// is shorter) and calls `on_chunk` per group, in order, before
    /// resolving with the same `Completion` `complete` builds.
    fn complete_streaming(
        &self,
        req: CompletionRequest,
        on_chunk: ChunkFn,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let reply = self.reply.clone();
        let fut = self.complete(req);
        Box::pin(async move {
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
                on_chunk(&s);
            }
            fut.await
        })
    }
}

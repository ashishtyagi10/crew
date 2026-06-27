use std::future::Future;
use std::pin::Pin;

use super::{Completion, CompletionRequest, Provider, ProviderError};

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
}

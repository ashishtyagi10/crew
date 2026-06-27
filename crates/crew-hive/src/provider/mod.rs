//! LLM provider abstraction: a `Provider` turns a prompt into a `Completion`.
//! Object-safe (boxed future, no async-trait) so the mock and the real
//! Anthropic client share one interface.
mod anthropic;
mod mock;
#[cfg(test)]
mod tests;

pub use anthropic::AnthropicProvider;
pub use mock::MockProvider;

use std::future::Future;
use std::pin::Pin;

#[derive(Clone, Debug)]
pub struct CompletionRequest {
    pub model: String,
    pub system: Option<String>,
    pub prompt: String,
    pub max_tokens: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Completion {
    pub text: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug)]
pub enum ProviderError {
    Http(String),
    Decode(String),
    Api(String),
    MissingKey,
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::Http(s) => write!(f, "http error: {s}"),
            ProviderError::Decode(s) => write!(f, "decode error: {s}"),
            ProviderError::Api(s) => write!(f, "api error: {s}"),
            ProviderError::MissingKey => write!(f, "ANTHROPIC_API_KEY not set"),
        }
    }
}

impl std::error::Error for ProviderError {}

pub trait Provider: Send + Sync {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>>;
}

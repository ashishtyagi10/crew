//! An OpenAI-compatible chat-completions [`Provider`]. Defaults to OpenRouter
//! (`OPENROUTER_API_KEY`), and — because the wire shape is shared — also backs
//! any other OpenAI-compatible endpoint via [`OpenRouterProvider::with_endpoint`]
//! (e.g. Alibaba Cloud DashScope). Mirrors [`super::AnthropicProvider`] but
//! speaks the OpenAI request/response shape (a `messages` array,
//! `choices[].message`).
use std::future::Future;
use std::pin::Pin;

use super::openai_http::request_with_retry;
use super::{http_client, request_timeout, Completion, CompletionRequest, Provider, ProviderError};

const ENDPOINT: &str = "https://openrouter.ai/api/v1/chat/completions";

/// Cheap to clone (the `reqwest::Client` is an `Arc` internally; the key is a
/// short `String`).
#[derive(Clone)]
pub struct OpenRouterProvider {
    client: reqwest::Client,
    api_key: String,
    /// The chat-completions URL — OpenRouter by default, any OpenAI-compatible
    /// endpoint via [`Self::with_endpoint`].
    endpoint: String,
    /// Fallback model chain: when the requested model is rate-limited or
    /// unavailable, `complete` advances to the next slug here. Empty = no
    /// fallback (just the request's own model, with transient retry).
    fallbacks: Vec<String>,
}

/// The ordered, de-duplicated models to try for one request: the requested model
/// first, then each configured fallback not already present. Because free
/// OpenRouter models route to *different* upstream providers, a different slug
/// often dodges a provider-specific throttle even when the account-wide daily
/// cap is shared.
fn attempt_chain(primary: &str, fallbacks: &[String]) -> Vec<String> {
    let mut chain = vec![primary.to_string()];
    for m in fallbacks {
        if !m.is_empty() && !chain.contains(m) {
            chain.push(m.clone());
        }
    }
    chain
}

impl OpenRouterProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: http_client(request_timeout()),
            api_key,
            endpoint: ENDPOINT.to_string(),
            fallbacks: Vec::new(),
        }
    }

    /// Point the provider at a different OpenAI-compatible chat-completions
    /// URL (e.g. DashScope's compatible mode).
    pub fn with_endpoint(mut self, url: impl Into<String>) -> Self {
        self.endpoint = url.into();
        self
    }

    /// Replace the per-attempt HTTP timeout (default: [`request_timeout`]).
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.client = http_client(timeout);
        self
    }

    /// Set the fallback model chain (see [`OpenRouterProvider::fallbacks`]). The
    /// request's own model is always tried first; these are the alternates.
    pub fn with_fallbacks(mut self, models: Vec<String>) -> Self {
        self.fallbacks = models;
        self
    }

    pub fn from_env() -> Result<Self, ProviderError> {
        match std::env::var("OPENROUTER_API_KEY") {
            Ok(k) if !k.is_empty() => Ok(Self::new(k)),
            _ => Err(ProviderError::MissingKey),
        }
    }
}

impl Provider for OpenRouterProvider {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let client = self.client.clone();
        let key = self.api_key.clone();
        let endpoint = self.endpoint.clone();
        let chain = attempt_chain(&req.model, &self.fallbacks);
        Box::pin(async move {
            let mut messages = Vec::new();
            if let Some(sys) = &req.system {
                messages.push(serde_json::json!({"role": "system", "content": sys}));
            }
            messages.push(serde_json::json!({"role": "user", "content": req.prompt}));
            // Try each model in turn; a model that stays rate-limited or is
            // unavailable (retired free slug, upstream error) hands off to the
            // next. Only a missing key short-circuits — no model can fix that.
            let mut last_err = ProviderError::Api("no model attempted".into());
            for model in &chain {
                let body = serde_json::json!({
                    "model": model,
                    "max_tokens": req.max_tokens,
                    "messages": messages,
                });
                match request_with_retry(&client, &endpoint, &key, &body).await {
                    Ok(c) => return Ok(c),
                    Err(ProviderError::MissingKey) => return Err(ProviderError::MissingKey),
                    Err(e) => last_err = e,
                }
            }
            Err(last_err)
        })
    }
}

#[cfg(test)]
#[path = "openrouter_tests.rs"]
mod tests;

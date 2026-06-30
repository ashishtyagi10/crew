//! An [`Provider`] backed by OpenRouter's OpenAI-compatible chat-completions
//! API, so the inbuilt agents can run on any OpenRouter-hosted model via
//! `OPENROUTER_API_KEY`. Mirrors [`super::AnthropicProvider`] but speaks the
//! OpenAI request/response shape (a `messages` array, `choices[].message`).
use std::future::Future;
use std::pin::Pin;

use serde::Deserialize;

use super::{Completion, CompletionRequest, Provider, ProviderError};

const ENDPOINT: &str = "https://openrouter.ai/api/v1/chat/completions";

/// Cheap to clone (the `reqwest::Client` is an `Arc` internally; the key is a
/// short `String`).
#[derive(Clone)]
pub struct OpenRouterProvider {
    client: reqwest::Client,
    api_key: String,
}

#[derive(Deserialize, Default)]
struct Msg {
    #[serde(default)]
    content: String,
}

#[derive(Deserialize)]
struct Choice {
    #[serde(default)]
    message: Msg,
}

#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

#[derive(Deserialize)]
struct ApiResp {
    #[serde(default)]
    choices: Vec<Choice>,
    usage: Option<Usage>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

impl OpenRouterProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }

    pub fn from_env() -> Result<Self, ProviderError> {
        match std::env::var("OPENROUTER_API_KEY") {
            Ok(k) if !k.is_empty() => Ok(Self::new(k)),
            _ => Err(ProviderError::MissingKey),
        }
    }

    pub(crate) fn parse_response(body: &str) -> Result<Completion, ProviderError> {
        let r: ApiResp =
            serde_json::from_str(body).map_err(|e| ProviderError::Decode(e.to_string()))?;
        if r.error.is_some() {
            return Err(ProviderError::Api(body.to_string()));
        }
        let text = r
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();
        let usage = r
            .usage
            .ok_or_else(|| ProviderError::Decode("missing usage".into()))?;
        Ok(Completion {
            text,
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
        })
    }
}

impl Provider for OpenRouterProvider {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let client = self.client.clone();
        let key = self.api_key.clone();
        Box::pin(async move {
            let mut messages = Vec::new();
            if let Some(sys) = &req.system {
                messages.push(serde_json::json!({"role": "system", "content": sys}));
            }
            messages.push(serde_json::json!({"role": "user", "content": req.prompt}));
            let body = serde_json::json!({
                "model": req.model,
                "max_tokens": req.max_tokens,
                "messages": messages,
            });
            let resp = client
                .post(ENDPOINT)
                .header("authorization", format!("Bearer {key}"))
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::Http(e.to_string()))?;
            let text = resp
                .text()
                .await
                .map_err(|e| ProviderError::Http(e.to_string()))?;
            OpenRouterProvider::parse_response(&text)
        })
    }
}

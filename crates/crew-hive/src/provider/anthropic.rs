use std::future::Future;
use std::pin::Pin;

use serde::Deserialize;

use super::{Completion, CompletionRequest, Provider, ProviderError};

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize)]
struct ApiResp {
    #[serde(default)]
    content: Vec<Block>,
    usage: Option<Usage>,
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }

    pub fn from_env() -> Result<Self, ProviderError> {
        match std::env::var("ANTHROPIC_API_KEY") {
            Ok(k) if !k.is_empty() => Ok(Self::new(k)),
            _ => Err(ProviderError::MissingKey),
        }
    }

    pub(crate) fn parse_response(body: &str) -> Result<Completion, ProviderError> {
        let r: ApiResp =
            serde_json::from_str(body).map_err(|e| ProviderError::Decode(e.to_string()))?;
        if r.kind == "error" || r.error.is_some() {
            return Err(ProviderError::Api(body.to_string()));
        }
        let text = r
            .content
            .iter()
            .find(|b| b.kind == "text")
            .map(|b| b.text.clone())
            .unwrap_or_default();
        let usage = r
            .usage
            .ok_or_else(|| ProviderError::Decode("missing usage".into()))?;
        Ok(Completion {
            text,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
        })
    }
}

impl Provider for AnthropicProvider {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let client = self.client.clone();
        let key = self.api_key.clone();
        Box::pin(async move {
            let mut body = serde_json::json!({
                "model": req.model,
                "max_tokens": req.max_tokens,
                "messages": [{"role": "user", "content": req.prompt}],
            });
            if let Some(sys) = &req.system {
                body["system"] = serde_json::json!(sys);
            }
            let resp = client
                .post(ENDPOINT)
                .header("x-api-key", key)
                .header("anthropic-version", VERSION)
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::Http(e.to_string()))?;
            let text = resp
                .text()
                .await
                .map_err(|e| ProviderError::Http(e.to_string()))?;
            AnthropicProvider::parse_response(&text)
        })
    }
}

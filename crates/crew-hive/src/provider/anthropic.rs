use std::future::Future;
use std::pin::Pin;

use serde::Deserialize;

use super::{
    http_client, request_timeout, Completion, CompletionRequest, Provider, ProviderError,
    ToolInvocation, Turn,
};

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const VERSION: &str = "2023-06-01";

/// Cloning is cheap: `reqwest::Client` is an `Arc` internally (shares one
/// connection pool) and the key is a short `String`. Sharing one provider
/// between the planner and the worker factory relies on this.
#[derive(Clone)]
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
    // `tool_use` blocks. Defaulted rather than in a second struct so one
    // `content` array parses whatever mix of blocks a reply carries.
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    input: Option<serde_json::Value>,
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
            client: http_client(request_timeout()),
            api_key,
        }
    }

    pub fn from_env() -> Result<Self, ProviderError> {
        Self::from_key(std::env::var("ANTHROPIC_API_KEY").ok())
    }

    /// [`Self::from_env`]'s decision, with the value passed in.
    ///
    /// Split out so the rule can be TESTED. The test used to read the process
    /// environment and assert only when the key happened to be absent, which
    /// means it asserted nothing at all on any machine that had one — the
    /// machines most likely to be running it.
    pub fn from_key(key: Option<String>) -> Result<Self, ProviderError> {
        match key {
            Some(k) if !k.is_empty() => Ok(Self::new(k)),
            _ => Err(ProviderError::MissingKey("ANTHROPIC_API_KEY")),
        }
    }

    pub(crate) fn parse_response(body: &str) -> Result<Completion, ProviderError> {
        let r: ApiResp =
            serde_json::from_str(body).map_err(|e| ProviderError::Decode(e.to_string()))?;
        if r.kind == "error" || r.error.is_some() {
            return Err(ProviderError::Api(body.to_string()));
        }
        // EVERY text block, joined — not just the first. A reply that thinks,
        // calls a tool, then thinks again carries several, and taking only
        // `find` silently dropped the rest.
        let text = r
            .content
            .iter()
            .filter(|b| b.kind == "text")
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join("");
        let calls: Vec<ToolInvocation> = r
            .content
            .iter()
            .filter(|b| b.kind == "tool_use")
            .map(|b| ToolInvocation {
                id: b.id.clone(),
                name: b.name.clone(),
                input: b.input.clone().unwrap_or_else(|| serde_json::json!({})),
            })
            .collect();
        let usage = r
            .usage
            .ok_or_else(|| ProviderError::Decode("missing usage".into()))?;
        Ok(Completion {
            text,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cost_microusd: 0,
            calls,
        })
    }
}

/// The `messages` array for one request: the opening user prompt, then every
/// turn since.
///
/// Anthropic requires the assistant's `tool_use` blocks to be REPLAYED in the
/// history — a tool result whose `tool_use_id` has no matching call in a prior
/// assistant turn is rejected outright — so the conversation is rebuilt in
/// full on every request rather than sending only the newest exchange.
pub(crate) fn build_messages(req: &CompletionRequest) -> Vec<serde_json::Value> {
    let mut messages = vec![serde_json::json!({"role": "user", "content": req.prompt})];
    for turn in &req.turns {
        match turn {
            Turn::Assistant { text, calls } => {
                let mut content = Vec::new();
                if !text.is_empty() {
                    content.push(serde_json::json!({"type": "text", "text": text}));
                }
                for c in calls {
                    content.push(serde_json::json!({
                        "type": "tool_use",
                        "id": c.id,
                        "name": c.name,
                        "input": c.input,
                    }));
                }
                messages.push(serde_json::json!({"role": "assistant", "content": content}));
            }
            // Results go back as a USER turn — they are input to the model,
            // not something it said.
            Turn::ToolResults(results) => {
                let content: Vec<serde_json::Value> = results
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "type": "tool_result",
                            "tool_use_id": r.id,
                            "content": r.content,
                            "is_error": r.is_error,
                        })
                    })
                    .collect();
                messages.push(serde_json::json!({"role": "user", "content": content}));
            }
        }
    }
    messages
}

/// The tools array, or `None` when there are none. ABSENT, not empty: an
/// empty `tools: []` is a different request and some endpoints reject it.
pub(crate) fn build_tools(req: &CompletionRequest) -> Option<serde_json::Value> {
    (!req.tools.is_empty()).then(|| {
        req.tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.input_schema,
                })
            })
            .collect()
    })
}

impl Provider for AnthropicProvider {
    fn supports_tools(&self) -> bool {
        true
    }

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
                "messages": build_messages(&req),
            });
            if let Some(sys) = &req.system {
                body["system"] = serde_json::json!(sys);
            }
            if let Some(tools) = build_tools(&req) {
                body["tools"] = tools;
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

use serde::{Deserialize, Serialize};

// ── Anthropic format ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub(super) struct AnthropicRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: String,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AnthropicResponse {
    pub content: Vec<AnthropicContentBlock>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AnthropicContentBlock {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}

// ── OpenAI-compatible format (OpenRouter, etc.) ───────────────────

#[derive(Debug, Serialize)]
pub(super) struct OpenAiRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct OpenAiResponse {
    pub choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
pub(super) struct OpenAiChoice {
    pub message: ChatMessage,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    pub enabled: bool,
    /// "anthropic", "openrouter", or "openai-compatible"
    pub provider: String,
    /// Base URL for the API (e.g. "https://openrouter.ai/api/v1")
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    /// Environment variable name to read the API key from
    pub api_key_env: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "openrouter".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            model: "google/gemma-3-4b-it:free".to_string(),
            max_tokens: 4096,
            api_key_env: "OPENROUTER_API_KEY".to_string(),
        }
    }
}

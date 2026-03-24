use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

// ── Anthropic format ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

// ── OpenAI-compatible format (OpenRouter, etc.) ───────────────────

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: ChatMessage,
}

// ── Agent ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ApiProvider {
    Anthropic,
    OpenAiCompatible,
}

pub struct AiAgent {
    api_key: Option<String>,
    provider: ApiProvider,
    base_url: String,
    model: String,
    max_tokens: u32,
}

impl AiAgent {
    pub fn new(
        provider: &str,
        base_url: String,
        model: String,
        max_tokens: u32,
        api_key_env: &str,
    ) -> Self {
        let api_key = std::env::var(api_key_env).ok();
        let provider = match provider {
            "anthropic" => ApiProvider::Anthropic,
            _ => ApiProvider::OpenAiCompatible,
        };
        Self {
            api_key,
            provider,
            base_url,
            model,
            max_tokens,
        }
    }

    pub fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    pub fn provider(&self) -> &ApiProvider {
        &self.provider
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn max_tokens(&self) -> u32 {
        self.max_tokens
    }

    pub fn api_key_env_name(&self) -> &str {
        match self.provider {
            ApiProvider::Anthropic => "ANTHROPIC_API_KEY",
            ApiProvider::OpenAiCompatible => "OPENROUTER_API_KEY",
        }
    }

    /// Process a natural language query about files.
    pub async fn query(
        &self,
        user_query: &str,
        current_dir: &Path,
        files_context: &str,
    ) -> Result<String> {
        let api_key = match &self.api_key {
            Some(key) => key.clone(),
            None => {
                let env_name = self.api_key_env_name();
                return Ok(format!(
                    "AI assistant is not configured.\n\n\
                     To enable AI features, set your API key:\n\n\
                     export {}=your-api-key-here\n\n\
                     Default provider: OpenRouter (free models available)\n\
                     Get a free key at: https://openrouter.ai/keys\n\n\
                     Then restart farx.",
                    env_name
                ));
            }
        };

        let system_prompt = format!(
            "You are the AI assistant for Farx, a terminal file manager (FAR Manager clone). \
             Help the user manage files through natural language.\n\n\
             Current directory: {}\n\n\
             Files in current directory:\n{}\n\n\
             Provide concise, actionable responses. When suggesting file operations, \
             describe what commands or actions the user should take. \
             Format your response for a terminal display (keep lines under 80 chars).",
            current_dir.display(),
            files_context,
        );

        match self.provider {
            ApiProvider::Anthropic => {
                self.query_anthropic(&api_key, &system_prompt, user_query)
                    .await
            }
            ApiProvider::OpenAiCompatible => {
                self.query_openai_compatible(&api_key, &system_prompt, user_query)
                    .await
            }
        }
    }

    async fn query_anthropic(
        &self,
        api_key: &str,
        system_prompt: &str,
        user_query: &str,
    ) -> Result<String> {
        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            system: system_prompt.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: user_query.to_string(),
            }],
        };

        let url = format!("{}/messages", self.base_url.trim_end_matches('/'));
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Ok(format!("API error ({}): {}", status, body));
        }

        let msg: AnthropicResponse = response.json().await?;
        let text = msg
            .content
            .iter()
            .filter_map(|block| block.text.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");

        if text.is_empty() {
            Ok("No response from AI.".to_string())
        } else {
            Ok(text)
        }
    }

    async fn query_openai_compatible(
        &self,
        api_key: &str,
        system_prompt: &str,
        user_query: &str,
    ) -> Result<String> {
        // Some free models don't support system messages, so we try with system first
        // and fall back to merging into user message if that fails.
        let messages_with_system = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_query.to_string(),
            },
        ];

        let messages_merged = vec![ChatMessage {
            role: "user".to_string(),
            content: format!(
                "{}\n\nUser request: {}",
                system_prompt, user_query
            ),
        }];

        // Try with system message first
        let request = OpenAiRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages: messages_with_system,
        };

        let url = format!(
            "{}/chat/completions",
            self.base_url.trim_end_matches('/')
        );
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("content-type", "application/json")
            // OpenRouter-specific headers (ignored by other providers)
            .header("HTTP-Referer", "https://github.com/farx-fm/farx")
            .header("X-Title", "Farx File Manager")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            // Retry with merged messages (no system role) for models that don't support it
            let retry_request = OpenAiRequest {
                model: self.model.clone(),
                max_tokens: self.max_tokens,
                messages: messages_merged,
            };

            let retry_response = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .header("HTTP-Referer", "https://github.com/farx-fm/farx")
                .header("X-Title", "Farx File Manager")
                .json(&retry_request)
                .send()
                .await?;

            if !retry_response.status().is_success() {
                let status = retry_response.status();
                let body = retry_response.text().await.unwrap_or_default();
                return Ok(format!("API error ({}): {}", status, body));
            }

            let msg: OpenAiResponse = retry_response.json().await?;
            let text = msg
                .choices
                .first()
                .map(|c| c.message.content.clone())
                .unwrap_or_default();
            return if text.is_empty() {
                Ok("No response from AI.".to_string())
            } else {
                Ok(text)
            };
        }

        let msg: OpenAiResponse = response.json().await?;
        let text = msg
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        if text.is_empty() {
            Ok("No response from AI.".to_string())
        } else {
            Ok(text)
        }
    }

    /// Build a context string from the current panel's files.
    pub fn build_files_context(entries: &[(String, bool, u64)]) -> String {
        let mut ctx = String::new();
        for (name, is_dir, size) in entries.iter().take(50) {
            if *is_dir {
                ctx.push_str(&format!("  [DIR] {}\n", name));
            } else {
                ctx.push_str(&format!("  {} ({} bytes)\n", name, size));
            }
        }
        if entries.len() > 50 {
            ctx.push_str(&format!(
                "  ... and {} more entries\n",
                entries.len() - 50
            ));
        }
        ctx
    }
}

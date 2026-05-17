use super::prompt::{build_system_prompt, not_configured_message};
use super::state::{AiAgent, ApiProvider};
use super::types::{
    AnthropicRequest, AnthropicResponse, ChatMessage, OpenAiRequest, OpenAiResponse,
};
use anyhow::Result;
use std::path::Path;

impl AiAgent {
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
                return Ok(not_configured_message(self.api_key_env_name()));
            }
        };

        let system_prompt = build_system_prompt(current_dir, files_context);

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

    pub(super) async fn query_anthropic(
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

    pub(super) async fn query_openai_compatible(
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
            content: format!("{}\n\nUser request: {}", system_prompt, user_query),
        }];

        let request = OpenAiRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages: messages_with_system,
        };

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("content-type", "application/json")
            .header("HTTP-Referer", "https://github.com/farx-fm/farx")
            .header("X-Title", "Farx File Manager")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
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
}

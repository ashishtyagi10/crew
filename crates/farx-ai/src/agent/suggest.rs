use super::prompt::build_suggest_prompt;
use super::state::AiAgent;
use super::types::{ChatMessage, OpenAiRequest, OpenAiResponse};
use anyhow::Result;
use std::path::Path;

impl AiAgent {
    /// Get a typeahead suggestion for partial command input.
    /// Returns just the completion text (not the full command).
    pub async fn suggest(
        &self,
        partial_input: &str,
        current_dir: &Path,
        files_context: &str,
    ) -> Result<Option<String>> {
        let api_key = match &self.api_key {
            Some(key) => key.clone(),
            None => return Ok(None),
        };

        let prompt = build_suggest_prompt(partial_input, current_dir, files_context);

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        let request = OpenAiRequest {
            model: self.model.clone(),
            max_tokens: 60,
            messages,
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
            .await;

        let response = match response {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        if !response.status().is_success() {
            return Ok(None);
        }

        let msg: OpenAiResponse = match response.json().await {
            Ok(m) => m,
            Err(_) => return Ok(None),
        };

        let text = msg
            .choices
            .first()
            .map(|c| c.message.content.trim().to_string())
            .unwrap_or_default();

        if text.is_empty() || text == "NONE" || text.contains('\n') {
            Ok(None)
        } else {
            Ok(Some(text))
        }
    }
}

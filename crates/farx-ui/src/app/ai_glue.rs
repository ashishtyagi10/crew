//! Async plumbing between the TUI and the AI agent: query submission,
//! response polling, and the debounced typeahead suggestion flow.

use super::App;

impl App {
    /// Submit an AI query in the background.
    pub(super) fn submit_ai_query(&mut self, query: String) {
        let current_dir = self.active_panel_ref().current_dir.clone();
        let entries: Vec<(String, bool, u64)> = self
            .active_panel_ref()
            .entries
            .iter()
            .map(|e| (e.name.clone(), e.is_dir, e.size))
            .collect();
        let files_context = farx_ai::AiAgent::build_files_context(&entries);

        let agent = farx_ai::AiAgent::new(
            &self.config.ai.provider,
            self.ai_agent.base_url().to_string(),
            self.ai_agent.model().to_string(),
            self.ai_agent.max_tokens(),
            &self.config.ai.api_key_env,
        );

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.ai_pending_response = Some(rx);

        tokio::spawn(async move {
            let result = agent.query(&query, &current_dir, &files_context).await;
            let response = match result {
                Ok(text) => text,
                Err(e) => format!("Error: {}", e),
            };
            let _ = tx.send(response);
        });
    }

    /// Request a typeahead suggestion from the LLM.
    pub(super) fn request_suggestion(&mut self) {
        let input = self.command_line.input.clone();
        if input.len() < 2 {
            return;
        }
        self.command_line.suggestion_pending = true;
        self.command_line.suggestion_for = input.clone();
        self.suggestion_request_input = input.clone();

        let dir = self.active_tree_ref().root.clone();
        let entries: Vec<(String, bool, u64)> = self
            .active_tree_ref()
            .visible_nodes
            .iter()
            .take(20)
            .map(|n| (n.entry.name.clone(), n.entry.is_dir, n.entry.size))
            .collect();
        let files_context = farx_ai::AiAgent::build_files_context(&entries);

        let agent = farx_ai::AiAgent::new(
            &self.config.ai.provider,
            self.ai_agent.base_url().to_string(),
            self.ai_agent.model().to_string(),
            self.ai_agent.max_tokens(),
            &self.config.ai.api_key_env,
        );

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.suggestion_rx = Some(rx);

        tokio::spawn(async move {
            let result = agent.suggest(&input, &dir, &files_context).await;
            let _ = tx.send(result.unwrap_or(None));
        });
    }

    /// Drain the suggestion channel (non-blocking).
    pub(super) fn check_suggestion_response(&mut self) {
        if let Some(ref mut rx) = self.suggestion_rx {
            match rx.try_recv() {
                Ok(suggestion) => {
                    if self.command_line.input == self.suggestion_request_input {
                        self.command_line.suggestion = suggestion;
                    }
                    self.command_line.suggestion_pending = false;
                    self.suggestion_rx = None;
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    self.command_line.suggestion_pending = false;
                    self.suggestion_rx = None;
                }
            }
        }
    }

    /// Drain the AI bar response channel (non-blocking).
    pub(super) fn check_ai_response(&mut self) {
        if let Some(ref mut rx) = self.ai_pending_response {
            match rx.try_recv() {
                Ok(response) => {
                    if let Some(ref mut ai_bar) = self.ai_bar {
                        ai_bar.set_response(response);
                    }
                    self.ai_pending_response = None;
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    if let Some(ref mut ai_bar) = self.ai_bar {
                        ai_bar.set_response("AI query was cancelled.".to_string());
                    }
                    self.ai_pending_response = None;
                }
            }
        }
    }
}

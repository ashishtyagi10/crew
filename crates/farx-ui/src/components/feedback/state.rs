//! Feedback system state and message API.

use std::time::{Duration, Instant};

use super::types::{ConfirmAction, FeedbackKind, FeedbackMessage, InlineConfirm};

/// The feedback system state
pub struct FeedbackState {
    /// Queue of messages (newest last)
    pub messages: Vec<FeedbackMessage>,
    /// Current inline confirmation, if any
    pub confirm: Option<InlineConfirm>,
    /// Scrollable output lines (for command output)
    pub output_lines: Vec<String>,
    pub output_scroll: usize,
    pub output_title: String,
    /// Whether output panel is visible
    pub output_visible: bool,
}

impl FeedbackState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            confirm: None,
            output_lines: Vec::new(),
            output_scroll: 0,
            output_title: String::new(),
            output_visible: false,
        }
    }

    /// Push a success message (auto-dismiss 3s)
    pub fn success(&mut self, text: impl Into<String>) {
        self.push(FeedbackKind::Success, text.into(), Duration::from_secs(3));
    }

    /// Push an error message (auto-dismiss 5s)
    pub fn error(&mut self, text: impl Into<String>) {
        self.push(FeedbackKind::Error, text.into(), Duration::from_secs(5));
    }

    /// Push a warning message (auto-dismiss 4s)
    pub fn warning(&mut self, text: impl Into<String>) {
        self.push(FeedbackKind::Warning, text.into(), Duration::from_secs(4));
    }

    /// Push an info message (auto-dismiss 3s)
    pub fn info(&mut self, text: impl Into<String>) {
        self.push(FeedbackKind::Info, text.into(), Duration::from_secs(3));
    }

    fn push(&mut self, kind: FeedbackKind, text: String, ttl: Duration) {
        // Only keep last 5 messages
        if self.messages.len() >= 5 {
            self.messages.remove(0);
        }
        self.messages.push(FeedbackMessage {
            kind,
            text,
            created: Instant::now(),
            ttl,
        });
    }

    /// Show scrollable output (e.g. command results)
    pub fn show_output(&mut self, title: impl Into<String>, text: String) {
        self.output_title = title.into();
        self.output_lines = text.lines().map(String::from).collect();
        self.output_scroll = 0;
        self.output_visible = true;
    }

    /// Request an inline confirmation
    pub fn ask_confirm(
        &mut self,
        prompt: impl Into<String>,
        detail: impl Into<String>,
        action: ConfirmAction,
    ) {
        self.confirm = Some(InlineConfirm {
            prompt: prompt.into(),
            detail: detail.into(),
            action_id: action,
            created: Instant::now(),
        });
    }

    /// Tick: remove expired messages, auto-dismiss output after inactivity
    pub fn tick(&mut self) {
        let now = Instant::now();
        self.messages
            .retain(|m| now.duration_since(m.created) < m.ttl);

        // Auto-dismiss output panel after 30 seconds
        if self.output_visible && !self.output_lines.is_empty() {
            // Keep it visible until user acts
        }
    }

    /// Check if feedback has anything to show
    pub fn has_content(&self) -> bool {
        !self.messages.is_empty() || self.confirm.is_some() || self.output_visible
    }

    /// Take the confirmed action (consumes it)
    pub fn take_confirm(&mut self) -> Option<ConfirmAction> {
        self.confirm.take().map(|c| c.action_id)
    }
}

impl Default for FeedbackState {
    fn default() -> Self {
        Self::new()
    }
}

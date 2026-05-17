//! Core data types for the feedback system.

use std::time::{Duration, Instant};

/// A single feedback message
#[derive(Debug, Clone)]
pub struct FeedbackMessage {
    pub kind: FeedbackKind,
    pub text: String,
    pub created: Instant,
    pub ttl: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FeedbackKind {
    /// Green success message
    Success,
    /// Red error message
    Error,
    /// Yellow warning message
    Warning,
    /// Cyan info message
    Info,
    /// Scrollable output (e.g. command output)
    Output,
}

/// Inline confirmation state
#[derive(Debug, Clone)]
pub struct InlineConfirm {
    pub prompt: String,
    pub detail: String,
    pub action_id: ConfirmAction,
    pub created: Instant,
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    Copy {
        sources: Vec<std::path::PathBuf>,
        dest: std::path::PathBuf,
    },
    Move {
        sources: Vec<std::path::PathBuf>,
        dest: std::path::PathBuf,
    },
    Delete {
        targets: Vec<std::path::PathBuf>,
    },
}

/// Result of handling a key in the feedback system
#[derive(Debug, Clone, PartialEq)]
pub enum FeedbackResult {
    /// Key was not consumed
    NotHandled,
    /// Key was consumed, no action needed
    Consumed,
    /// Confirmation was accepted
    Confirmed(usize), // index into confirm queue
    /// Confirmation was rejected
    Rejected,
}

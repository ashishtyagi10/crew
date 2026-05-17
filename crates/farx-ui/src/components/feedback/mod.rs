//! Inline feedback system — replaces all modal dialogs.
//!
//! Messages appear in the command bar area and auto-dismiss.
//! Confirmations are inline Y/N prompts that resolve without blocking.

mod keys;
mod output_panel;
mod render;
mod state;
mod types;

pub use render::render_feedback;
pub use state::FeedbackState;
pub use types::{ConfirmAction, FeedbackKind, FeedbackMessage, FeedbackResult, InlineConfirm};

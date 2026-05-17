//! Dialog component module.
//!
//! Provides modal dialog state (input, confirm, message, error variants),
//! key handling, and rendering helpers.

mod keys;
mod render;
mod state;
mod variants;

pub use render::render_dialog;
pub use state::{DialogKind, DialogResult, DialogState};

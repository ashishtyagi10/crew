//! Command line component: input state, history, suggestions, rendering.

mod history;
mod input;
mod render;
mod state;

pub use render::render_command_line;
pub use state::CommandLineState;

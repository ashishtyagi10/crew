//! Agent grid LRU: tracks pane indices in most-recently-active order, caps the
//! number of full tiles, and demotes the rest to a minimized strip. Pure and
//! UI-independent; `build_frame` consumes it to place panes. See
//! `compute`/`compose_grid` for turning this state into pixel rects.

mod state;

#[cfg(test)]
mod tests;

pub use state::{GridLayout, MAX_FULL_TILES};

//! Agent grid LRU: tracks pane indices in most-recently-active order, caps the
//! number of full tiles, and demotes the rest to a minimized strip. Pure and
//! UI-independent; `build_frame` consumes it to place panes.

mod compose;
mod state;

#[cfg(test)]
mod tests;

pub use compose::compose_grid;
pub use state::GridLayout;

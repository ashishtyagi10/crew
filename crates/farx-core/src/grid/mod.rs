//! Agent grid layout: pack N tiles into a near-square grid and track which
//! tiles are shown full vs. minimized (LRU). UI-independent; consumed by the
//! renderer.

mod compose;
mod geometry;
mod state;

pub use compose::{compute_grid_layout, GridRects, MINIMIZED_STRIP_HEIGHT};
pub use geometry::{grid_rects, MAX_FULL_TILES};
pub use state::GridLayout;

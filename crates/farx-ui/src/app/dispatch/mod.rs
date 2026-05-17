//! Dispatch routing. The top-level `App::dispatch` is in `mod.rs` (parent
//! module); this submodule holds the per-category prefix routers that the
//! parent walks before falling through to the main action match.

mod selection;
mod tree_nav;

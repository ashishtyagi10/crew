//! The file viewer pane: one zoomed, read-only pane over a ladder of formats.
//! `detect` classifies bytes, `load` fetches them off the winit thread, and
//! every rung renders down to the same `Vec<CardLine>` the chat cards use.
pub(crate) mod detect;
pub(crate) mod load;
pub(crate) mod mdrung;
mod pane;
pub(crate) use pane::{LoadState, ViewCache, ViewPane};

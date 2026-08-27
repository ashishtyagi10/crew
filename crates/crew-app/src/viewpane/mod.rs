//! The file viewer pane: one zoomed, read-only pane over a ladder of formats.
//! `detect` classifies bytes, `load` fetches them off the winit thread, and
//! every rung renders down to the same `Vec<CardLine>` the chat cards use.
mod codepaint;
pub(crate) mod csv;
pub(crate) mod detect;
mod diffpaint;
pub(crate) mod keys;
mod lines;
pub(crate) mod load;
pub(crate) mod mdrung;
mod metacard;
mod pane;
mod render;
mod rendercap;
pub(crate) mod search;
mod search_apply;
pub(crate) use keys::ViewAction;
pub(crate) use pane::{LoadState, ViewCache, ViewPane};

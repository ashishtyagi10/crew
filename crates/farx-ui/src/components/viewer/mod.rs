mod follow;
mod hex;
mod keys;
mod mouse;
mod render;
mod render_status;
mod search;
mod state;

pub use render::render_viewer;
pub use state::ViewerState;

#[derive(Debug, Clone, PartialEq)]
pub enum ViewerAction {
    None,
    Close,
}

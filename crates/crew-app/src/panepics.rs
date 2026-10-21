//! The pictures a pane's content draws as textures (`crew_render::Picture`),
//! beside the cells and paint `Pane::art` gives: the open mail's, for now.
use crate::pane::{Pane, PaneContent};

/// Pane `p`'s pictures; `aspect` is `cell_h / cell_w`.
pub(crate) fn of(p: &Pane, aspect: f32) -> Vec<crew_render::Picture> {
    match &p.content {
        PaneContent::Inbox(i) => i.pictures(p.grid.cols, p.grid.rows, aspect),
        _ => Vec::new(),
    }
}

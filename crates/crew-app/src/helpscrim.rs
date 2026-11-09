//! The `/keys` overlay sets everything under it back.
//!
//! The keys panel is a modal: while it is up the keys go to it (`keys.rs`),
//! not to the panes or the bar. It used to float over a grid still at full
//! ink — the focused pane brightest of all, right behind the panel — and the
//! card under the pointer still rose toward it, so the thing you could not
//! touch was the thing that answered the pointer. Now the panel is the only
//! lit surface: every scene under it leans toward the page, and no card
//! lifts for a pointer the panes are not listening to.
//!
//! Split from `render.rs` for the line cap.
use crew_render::PaneScene;

/// How far the ink under the panel leans toward the page — about focus
/// mode's wash (`focusmode::DIM`), so the grid stays legible as context
/// (you read the panel against the pane you opened it from) while the panel
/// is plainly in front. On see-through glass `spotlight::wash` stops the
/// lean at the UI floor, so words dim there without dropping under it.
pub(crate) const DIM: f32 = 0.45;

/// Wash every scene already pushed for this frame when the panel is open.
/// Called after the panes, nav and bar are pushed and before the panel, so
/// the panel (and toasts, which sit above it) keep full ink.
pub(crate) fn apply(scenes: &mut [PaneScene], help_open: bool) {
    if !help_open {
        return;
    }
    let dim = DIM * crew_theme::contrast::effect_scale();
    for s in scenes {
        crate::spotlight::wash(&mut s.cells, dim);
    }
}

/// The card the pointer may lift: none while the panel is up — the panes
/// take no clicks or keys then, so nothing under the panel should rise.
pub(crate) fn hover(help_open: bool, pane: Option<usize>) -> Option<usize> {
    pane.filter(|_| !help_open)
}

#[cfg(test)]
#[path = "helpscrim_tests.rs"]
mod tests;

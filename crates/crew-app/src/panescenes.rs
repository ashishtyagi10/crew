//! Assembling a frame's scenes: the full pass over every pane, and the
//! budgeted one that scans only what changed.
//!
//! Split from [`crate::paneview`] for the line cap, along the line between
//! deciding WHICH panes to draw and pushing one pane's own scenes.
use crate::gridsel::CellSel;
use crate::pane::Pane;
use crate::paneview::push_pane_scenes;
use crew_render::PaneScene;

/// Build the `PaneScene`s for one frame. Each pane yields **two** scenes — the
/// content, inset by one cell on every side, and the border card around it —
/// kept in separate text buffers so the box-drawing border glyphs never share a
/// line with (and so never shift) the content. `broadcast` marks terminal panes
/// receiving synchronized input; `find` is the active `/find` term, highlighted
/// in the focused pane while scrolled back.
#[allow(clippy::too_many_arguments)]
pub fn build_scenes(
    panes: &[Pane],
    focused: Option<usize>,
    broadcast: bool,
    find: Option<&str>,
    sel: Option<&CellSel>,
    focus_t: f32,
    cw: f32,
    ch: f32,
    git: &crate::gitfleet::GitFleet,
    pinned: &[usize],
) -> Vec<PaneScene> {
    let multi = panes.len() > 1;
    let mut scenes = Vec::with_capacity(panes.len() * 2);
    for (i, p) in panes.iter().enumerate() {
        let foc = focused == Some(i);
        // Slice-rebased (zoom, harnesses): pane 0 is the spotlit one.
        let dim = crate::spotlight::dim_for(i, 0, 0, focus_t);
        // This slice is index-rebased (zoom renders a 1-pane slice), so the
        // selection — keyed by absolute index — is matched to the focused pane.
        // The minimize button rides the zoomed border too: hit-testing shares
        // the drawn rect (render::frame_hit_rects), so the click region lands
        // on the glyphs exactly.
        push_pane_scenes(
            &mut scenes,
            p,
            i,
            multi.then_some(i + 1),
            foc,
            broadcast,
            find,
            foc.then_some(sel).flatten(),
            true,
            focus_t,
            dim,
            cw,
            ch,
            git,
            pinned.contains(&i),
        );
    }
    scenes
}

/// Render the panes named by `placed` (`(pane_index, rect)`), numbering tiles
/// by pane index so badges match `Cmd+N` and the sidebar. `focused` is the
/// *pane index* of the focused pane.
/// Callers must have applied `relayout_one` to each placed full pane first
/// (build_frame does this) — this reads `pane.rect`.
#[allow(clippy::too_many_arguments)]
pub fn full_scenes(
    panes: &[Pane],
    placed: &[(usize, crate::layout::Rect)],
    focused: Option<usize>,
    broadcast: bool,
    find: Option<&str>,
    sel: Option<&CellSel>,
    focus_t: f32,
    // Spotlight `(spot, prev)`: which pane holds the light and which it just
    // left — follows `app.focused` even while the input bar owns the keys.
    spot: (usize, usize),
    cw: f32,
    ch: f32,
    git: &crate::gitfleet::GitFleet,
    pinned: &[usize],
) -> Vec<PaneScene> {
    let mut scenes = Vec::with_capacity(placed.len() * 2);
    for &(idx, _rect) in placed {
        let p = &panes[idx];
        let foc = focused == Some(idx);
        let dim = crate::spotlight::dim_for(idx, spot.0, spot.1, focus_t);
        push_pane_scenes(
            &mut scenes,
            p,
            idx,
            (panes.len() > 1).then_some(idx + 1),
            foc,
            broadcast,
            find,
            sel.filter(|s| s.pane == idx),
            true,
            focus_t,
            dim,
            cw,
            ch,
            git,
            pinned.contains(&idx),
        );
    }
    scenes
}

//! Assembling panes into `PaneScene`s for `renderer.frame`. Each pane is a
//! fieldset card (see [`crate::panecard`]): the content and its rounded border
//! ride separate text buffers so the border never shifts the content.
use crew_render::PaneScene;

use crate::gridsel::CellSel;
use crate::pane::{Pane, PaneContent};
use crate::panecard::{pane_card, Bar};

/// Build the `PaneScene`s for one frame. Each pane yields **two** scenes — the
/// content, inset by one cell on every side, and the border card around it —
/// kept in separate text buffers so the box-drawing border glyphs never share a
/// line with (and so never shift) the content. `broadcast` marks terminal panes
/// receiving synchronized input; `find` is the active `/find` term, highlighted
/// in the focused pane while scrolled back.
pub fn build_scenes(
    panes: &[Pane],
    focused: Option<usize>,
    broadcast: bool,
    find: Option<&str>,
    sel: Option<&CellSel>,
    cw: f32,
    ch: f32,
) -> Vec<PaneScene> {
    let multi = panes.len() > 1;
    let mut scenes = Vec::with_capacity(panes.len() * 2);
    for (i, p) in panes.iter().enumerate() {
        let foc = focused == Some(i);
        // This slice is index-rebased (zoom renders a 1-pane slice), so the
        // selection — keyed by absolute index — is matched to the focused pane.
        // No minimize button here: this path renders the zoomed single pane
        // (and 1-pane slices), whose grid rects don't match the drawn rect, so
        // a border button would have no reliable hit region.
        push_pane_scenes(
            &mut scenes,
            p,
            multi.then_some(i + 1),
            foc,
            broadcast,
            find,
            foc.then_some(sel).flatten(),
            false,
            cw,
            ch,
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
    cw: f32,
    ch: f32,
) -> Vec<PaneScene> {
    let mut scenes = Vec::with_capacity(placed.len() * 2);
    for &(idx, _rect) in placed {
        let p = &panes[idx];
        let foc = focused == Some(idx);
        push_pane_scenes(
            &mut scenes,
            p,
            (panes.len() > 1).then_some(idx + 1),
            foc,
            broadcast,
            find,
            sel.filter(|s| s.pane == idx),
            true,
            cw,
            ch,
        );
    }
    scenes
}

/// Whether a pane is doing background work, so its border shows the
/// indeterminate progress sweep (swarm planning/running, agent chat awaiting).
pub(crate) fn pane_busy(p: &Pane) -> bool {
    match &p.content {
        PaneContent::Swarm(s) => s.is_busy(),
        PaneContent::Chat(c) => c.is_busy(),
        PaneContent::Far(f) => f.is_busy(),
        _ => false,
    }
}

/// Busy or briefly animating (a message card fading in): the redraw-scheduling
/// predicate for `poll` — wider than [`pane_busy`], which alone decides the
/// card's busy sweep so a fade never reads as "working".
pub(crate) fn pane_animating(p: &Pane) -> bool {
    pane_busy(p)
        || match &p.content {
            PaneContent::Chat(c) => c.is_fading(),
            _ => false,
        }
}

#[allow(clippy::too_many_arguments)]
fn push_pane_scenes(
    scenes: &mut Vec<PaneScene>,
    p: &Pane,
    index: Option<usize>,
    foc: bool,
    broadcast: bool,
    find: Option<&str>,
    sel: Option<&CellSel>,
    min_btn: bool,
    cw: f32,
    ch: f32,
) {
    let mut cells = p.cells(foc);
    let is_term = matches!(&p.content, PaneContent::Terminal(_));
    let scroll = match &p.content {
        PaneContent::Terminal(t) => t.pty.display_offset(),
        _ => 0,
    };
    // Tint http(s) URLs blue so they read as clickable (Cmd+click opens).
    if is_term {
        crate::linkhl::colorize(&mut cells, p.grid.cols, p.grid.rows);
    }
    // Wash search matches in the focused terminal while viewing a /find
    // result (scrolled back); it self-clears on return to the bottom.
    if foc && is_term && scroll > 0 {
        if let Some(term) = find {
            crate::findhl::highlight(&mut cells, term, p.grid.cols, p.grid.rows);
        }
    }
    // Wash a generic mouse selection over a non-terminal pane (terminals carry
    // their selection in the cell data already).
    if let Some(s) = sel {
        crate::gridsel::highlight(&mut cells, s, crew_theme::theme().find_hl_bg);
    }
    let r = p.rect;
    // Content: its own buffer, inset one cell past the top-left border so it
    // starts exactly on the grid (no leading border glyph to push it).
    scenes.push(PaneScene {
        cells,
        x: r.x + cw,
        y: r.y + ch,
        w: (r.w - 2.0 * cw).max(0.0),
        h: (r.h - 2.0 * ch).max(0.0),
        focused: foc,
        bordered: false,
        overlay: false,
    });
    // Border card: the rounded frame + legend + status, drawn over the rect.
    let title = p.title_text();
    scenes.push(PaneScene {
        cells: pane_card(
            p.grid.cols,
            p.grid.rows,
            &Bar {
                index,
                title: &title,
                focused: foc,
                scroll,
                activity: p.activity && !foc,
                bell: p.bell && !foc,
                broadcast: broadcast && is_term,
                busy: pane_busy(p).then(crate::anim::now_ms),
                min_btn,
            },
        ),
        x: r.x,
        y: r.y,
        w: r.w,
        h: r.h,
        focused: foc,
        bordered: false,
        overlay: false,
    });
}

//! Assembling panes into `PaneScene`s for `renderer.frame`. Each pane is a
//! fieldset card (see [`crate::panecard`]): the content and its rounded border
//! ride separate text buffers so the border never shifts the content.
use crew_render::PaneScene;

use crate::gridsel::CellSel;
use crate::pane::{Pane, PaneContent};
use crate::panecard::Bar;

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

/// How long a card takes to draw itself in.
const ASSEMBLE_MS: u64 = 380;

/// Period of the busy scan's round trip down a working card and back.
const SCAN_MS: u64 = 2_600;

/// This pane's assemble timeline. Scaled by the Motion setting, which is read
/// here rather than threaded through every scene call — at `off` the timeline
/// is born settled and the card is simply drawn.
pub(crate) fn spawn_timeline(p: &Pane) -> crate::ease::Timeline {
    crate::ease::Timeline::start(p.born_ms, ASSEMBLE_MS, crate::motion::level())
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
    focus_t: f32,
    dim: f32,
    cw: f32,
    ch: f32,
    git: &crate::gitfleet::GitFleet,
    pinned: bool,
) {
    let mut cells = p.cells(foc);
    let is_term = matches!(&p.content, PaneContent::Terminal(_));
    let (scroll, total) = match &p.content {
        PaneContent::Terminal(t) => (t.pty.display_offset(), t.pty.scrollable_lines()),
        // A document is longer than its pane far more often than a shell is,
        // and it had no position indicator at all.
        PaneContent::View(v) => v.position(p.grid.cols, p.grid.rows),
        _ => (0, 0),
    };
    let ticks = match &p.content {
        PaneContent::View(v) => v.mark_rows(p.grid.cols),
        _ => Vec::new(),
    };
    let hits = match &p.content {
        PaneContent::View(v) => v.hit_rows(),
        _ => Vec::new(),
    };
    // Where the commands you ran began, in this window.
    let cmd_rows = match &p.content {
        PaneContent::Terminal(t) => t.spans.start_rows(
            t.pty.scrollable_lines(),
            usize::from(p.grid.rows),
            t.pty.display_offset(),
        ),
        _ => Vec::new(),
    };
    // Rows of this pane's visible output that read as errors. Computed from
    // the cells already built for the frame rather than by re-reading the
    // grid — one pass over what is on screen.
    let err_rows = match is_term {
        true => crate::errscan::error_rows(&crate::gridrows::grid_lines(
            &cells,
            p.grid.cols,
            p.grid.rows,
        )),
        false => Vec::new(),
    };
    // A pane's foreground command and how long it has been at it. `cmd_since`
    // is stamped when the command starts and cleared when it ends, so an idle
    // shell has nothing to report.
    let elapsed = match &p.content {
        PaneContent::Terminal(t) => t
            .cmd_since
            .map(|since| since.elapsed())
            .and_then(crate::runclock::label),
        _ => None,
    };
    let progress = match &p.content {
        PaneContent::Terminal(t) => t.pty.progress(),
        _ => None,
    };
    let unread = match &p.content {
        PaneContent::Terminal(t) => crate::unread::count(t.pty.scrollable_lines(), t.read_at),
        _ => 0,
    };
    // The boundary between what you had read in this pane and what arrived
    // while you were reading another one.
    if let PaneContent::Terminal(t) = &p.content {
        let total = t.pty.scrollable_lines();
        let row = crate::unread::divider_row(
            total,
            t.read_at,
            usize::from(p.grid.rows),
            t.pty.display_offset(),
        );
        if let Some(row) = row {
            crate::unread::mark(&mut cells, row, p.grid.cols);
        }
    }
    // An agent cites files in prose as often as a compiler does, and a chat
    // pane resolves the same Cmd+click a terminal one does — so the same
    // marks belong here.
    if matches!(&p.content, PaneContent::Chat(_)) {
        crate::pathhl::mark(&mut cells, p.grid.cols, p.grid.rows);
    }
    // Mark what is clickable: file references first (dotted rule), then URLs
    // (solid) — a URL that also looks like a path is re-marked as the URL it
    // is, rather than wearing both rules.
    if is_term {
        crate::pathhl::mark(&mut cells, p.grid.cols, p.grid.rows);
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
    // Focus spotlight: unfocused content leans toward the page (frame cells
    // keep their own focus colors — this is about the ink, not the box).
    crate::spotlight::wash(&mut cells, dim);
    // The card draws itself in over its first moments. Read from the pane's own
    // birth stamp, so panes that appear together (a restored session) assemble
    // together, and one spawned later assembles on its own clock.
    let now = crate::anim::now_ms();
    let assemble_t = spawn_timeline(p).eased(now, crate::ease::out_cubic);
    // A working pane's sheet carries a scan sweeping down it. Gated on `busy`
    // and nothing else: a busy pane already repaints at ~15fps, so this costs
    // no extra frames, and an idle crew never draws a scan at all — which is
    // how an always-moving surface stays compatible with never repainting.
    let scan = match pane_busy(p) && crate::motion::level() != crate::motion::MotionLevel::Off {
        true => crate::anim::tri(now, SCAN_MS),
        false => -1.0,
    };
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
        glass: false,
        scan: -1.0,
        overlay: false,
    });
    // Border card: the rounded frame + legend + status, drawn over the rect.
    let title = p.title_text();
    scenes.push(PaneScene {
        cells: crate::panecardglow::pane_card_glowing(
            p,
            &Bar {
                index,
                title: &title,
                focused: foc,
                scroll,
                total,
                activity: p.activity && !foc,
                bell: p.bell && !foc,
                broadcast: broadcast && is_term,
                min_btn,
                focus_t,
                assemble_t,
                git: git.info(p.dir.as_deref()),
                ticks: &ticks,
                hits: &hits,
                progress,
                elapsed,
                pinned,
                cmd_rows: &cmd_rows,
                err_rows: &err_rows,
                unread,
                doc: matches!(p.content, PaneContent::View(_)),
            },
        ),
        x: r.x,
        y: r.y,
        w: r.w,
        h: r.h,
        focused: foc,
        bordered: false,
        // The card scene spans the whole pane rect, so the frosted sheet goes
        // here rather than on the cell-inset content above.
        glass: true,
        scan,
        overlay: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::farpane::FarPane;
    use crate::layout::Rect;
    use crew_term::GridSize;

    /// The regression this file exists to prevent: the Glass setting renders
    /// nothing unless a scene the APP builds asks for the sheet. Gating it on
    /// `bordered` — which every real scene sets `false` — shipped a glass
    /// feature that drew a card only in its own headless harness. Assert
    /// against `build_scenes`, not a hand-built `PaneScene`.
    #[test]
    fn every_pane_gets_exactly_one_glass_card() {
        let scenes = build_scenes(
            &[test_pane(), test_pane()],
            Some(0),
            false,
            None,
            None,
            1.0,
            10.0,
            16.0,
            &Default::default(),
            &[],
        );
        let cards: Vec<_> = scenes.iter().filter(|s| s.glass).collect();
        assert_eq!(cards.len(), 2, "one frosted sheet per pane");
        // The sheet has to cover the whole pane, not the cell-inset content —
        // a sheet drawn on the content scene would leave the frame unfrosted.
        for c in cards {
            assert_eq!((c.w, c.h), (820.0, 416.0), "sheet must span the pane rect");
        }
    }

    fn test_pane() -> Pane {
        Pane {
            glide: crate::glide::Glide::default(),
            content: PaneContent::Far(FarPane::new(std::env::temp_dir())),
            grid: GridSize { cols: 80, rows: 24 },
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 820.0,
                h: 416.0,
            },
            label: None,
            name: Some("md".into()),
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
            born_ms: crate::anim::now_ms(),
        }
    }

    #[test]
    fn zoomed_scenes_carry_the_minimize_button() {
        let pane = Pane {
            glide: crate::glide::Glide::default(),
            content: PaneContent::Far(FarPane::new(std::env::temp_dir())),
            grid: GridSize { cols: 80, rows: 24 },
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 820.0,
                h: 416.0,
            },
            label: None,
            name: Some("md".into()),
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
            born_ms: crate::anim::now_ms(),
        };
        let scenes = build_scenes(
            &[pane],
            Some(0),
            false,
            None,
            None,
            1.0,
            10.0,
            16.0,
            &Default::default(),
            &[],
        );
        // scenes[1] is the border card; the [-][x] buttons sit at card columns
        // cols-8..=cols-6 and cols-5..=cols-3 on row 0 (cols = grid cols + 2 border cells).
        let cols = 80 + 2;
        let border = &scenes[1].cells;
        let at = |col: u16| {
            border
                .iter()
                .find(|c| c.row == 0 && c.col == col)
                .map(|c| c.c)
        };
        // The [-] minimize button
        assert_eq!(at(cols - 8), Some('['));
        assert_eq!(at(cols - 7), Some('-'));
        assert_eq!(at(cols - 6), Some(']'));
        // The [x] close button
        assert_eq!(at(cols - 5), Some('['));
        assert_eq!(at(cols - 4), Some('x'));
        assert_eq!(at(cols - 3), Some(']'));
    }
}

//! Assembling panes into `PaneScene`s for `renderer.frame`. Each pane is a
//! fieldset card (see [`crate::panecard`]): the content and its rounded border
//! ride separate text buffers so the border never shifts the content.
pub(crate) use crate::panescenes::*;

pub(crate) use crate::panebusy::*;
use crew_render::PaneScene;

use crate::gridsel::CellSel;
use crate::pane::{Pane, PaneContent};
use crate::panecard::Bar;

#[allow(clippy::too_many_arguments)]
pub(crate) fn push_pane_scenes(
    scenes: &mut Vec<PaneScene>,
    p: &Pane,
    // This pane's absolute index in `app.panes` — what the pointer's hover is
    // keyed by. Not `index`, which is the card's LEGEND number and is `None`
    // on a canvas holding one pane.
    pane: usize,
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
    // Cells and the sub-cell paint under them, in one pass — see `Pane::art`.
    let (mut cells, mut paint) = p.art(foc, ch / cw);
    // The caret's wake, over this pane's own paint: only the focused pane has
    // one, and it is drawn under the text so the glyph it crosses stays read.
    if foc {
        paint.extend(crate::cursortrail::paint_for(p, crate::anim::now_ms()));
    }
    // Pictures the program in this pane sent, anchored to the lines they
    // arrived on and clipped to the pane (see `termimg`).
    if let PaneContent::Terminal(t) = &p.content {
        paint.extend(t.images.paint(
            t.pty.history_lines(),
            t.pty.display_offset(),
            p.grid.cols,
            p.grid.rows,
            ch / cw,
        ));
    }
    let is_term = matches!(&p.content, PaneContent::Terminal(_));
    let (scroll, total) = match &p.content {
        PaneContent::Terminal(t) => (t.pty.display_offset(), t.pty.scrollable_lines()),
        // A document is longer than its pane far more often than a shell is,
        // and it had no position indicator at all.
        PaneContent::View(v) => v.position(p.grid.cols, p.grid.rows),
        // A transcript is a document too, and it is the pane most likely to
        // be longer than its window.
        PaneContent::Chat(c) => c.position(p.grid.cols, p.grid.rows),
        _ => (0, 0),
    };
    let ticks = match &p.content {
        PaneContent::View(v) => v.mark_rows(p.grid.cols),
        PaneContent::Chat(c) => c.turn_rows(p.grid.cols),
        _ => Vec::new(),
    };
    let hits = match &p.content {
        PaneContent::View(v) => v.hit_rows(),
        _ => Vec::new(),
    };
    // One read of this pane's cells, shared by everything that wants its
    // rows as text: the file-reference marks, the URL marks and the error
    // scan. Each used to build the same `Vec<Vec<char>>` from the same cells,
    // one after another, on every frame of every terminal pane.
    let text_rows = match is_term || matches!(&p.content, PaneContent::Chat(_)) {
        true => crate::gridrows::grid_lines(&cells, p.grid.cols, p.grid.rows),
        false => Vec::new(),
    };
    // Both border markings answer to one switch: they are crew drawing on
    // its own chrome about someone else's output, and a plain frame is a
    // reasonable thing to want.
    let marks = crate::bordermarks::on();
    // Where the commands you ran began, in this window.
    let cmd_rows = match &p.content {
        PaneContent::Terminal(t) if marks => t.spans.start_rows(
            t.pty.scrollable_lines(),
            usize::from(p.grid.rows),
            t.pty.display_offset(),
        ),
        _ => Vec::new(),
    };
    // While scrolled back, the command whose output the TOP of the window is
    // inside. Named on the top border beside the `⇡N` that appears under the
    // same condition — see `cmdhead`. Answered from the same spans the left
    // border ticks, so the tick ladder and the name can never disagree.
    let at_cmd = match &p.content {
        PaneContent::Terminal(t) if marks && t.pty.display_offset() > 0 => {
            let now = t.pty.scrollable_lines();
            let first = now
                .saturating_sub(usize::from(p.grid.rows))
                .saturating_sub(t.pty.display_offset());
            // A block the shell reported a failure for says so here too: the
            // left border marks WHERE it began, and while you are scrolled
            // inside it the name on the top border is what you are reading.
            t.spans.at_line(first, now).map(|s| match s.exit {
                Some(code) if code != 0 => format!("{} \u{2717}{code}", s.name),
                _ => s.name.clone(),
            })
        }
        _ => None,
    };
    // …and which of them the shell said went wrong (OSC 133).
    let fail_rows = match &p.content {
        PaneContent::Terminal(t) if marks => t.spans.failed_rows(
            t.pty.scrollable_lines(),
            usize::from(p.grid.rows),
            t.pty.display_offset(),
        ),
        _ => Vec::new(),
    };
    // Rows of this pane's visible output that read as errors. Computed from
    // the cells already built for the frame rather than by re-reading the
    // grid — one pass over what is on screen.
    let err_rows = match is_term && marks {
        true => crate::errscan::error_rows(&text_rows),
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
    // An agent cites files in prose as often as a compiler does, and a chat
    // pane resolves the same Cmd+click a terminal one does — so the same
    // marks belong here.
    if matches!(&p.content, PaneContent::Chat(_)) {
        crate::pathhl::mark_in(&mut cells, &text_rows);
    }
    // Mark what is clickable: file references first (dotted rule), then URLs
    // (solid) — a URL that also looks like a path is re-marked as the URL it
    // is, rather than wearing both rules.
    if is_term {
        crate::pathhl::mark_in(&mut cells, &text_rows);
        crate::linkhl::colorize_in(&mut cells, &text_rows);
    }
    // …and which one the pointer has found. After both markers, so the
    // weight lands on whatever they drew, and never instead of it.
    crate::linkhover::mark(&mut cells, pane);
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
    // Hint labels, if this is the labelled pane — after the wash, because the
    // tags are the one thing on the pane that must not be dimmed.
    crate::hints::mark_pane(&mut cells, pane);
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
        paint,
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
    let bar = Bar {
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
        at_cmd: at_cmd.as_deref(),
        cmd_rows: &cmd_rows,
        fail_rows: &fail_rows,
        err_rows: &err_rows,
        unread,
        doc: matches!(p.content, PaneContent::View(_) | PaneContent::Chat(_)),
    };
    // The card's own grid: the pane's content grid plus its border ring —
    // the same `(cols + 2, rows + 2)` `pane_card` lays its cells out on, so
    // the drawing and the frame agree about where the borders are.
    let (ccols, crows) = (p.grid.cols + 2, p.grid.rows + 2);
    scenes.push(PaneScene {
        cells: crate::panecardglow::pane_card_glowing(p, &bar),
        // The frame's continuous readings — the scroll thumb and the
        // program's progress bar — drawn rather than spelled.
        paint: crate::cardpaint::card_paint(ccols, crows, &bar, ch / cw, now),
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
#[path = "paneview_tests.rs"]
mod tests;

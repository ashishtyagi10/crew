//! Composes the crew pane's full cell view: the header row (row 0),
//! role-styled message cards, the input composer (bordered fieldset on tall
//! panes, a bare prompt row on short ones), and — on tall panes — the
//! whole-pane summary footer (`chatsummary`) on the very bottom row. Tiny panes
//! fall back to the plain layout.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatbody::CardCell;
use crate::chatlayout::layout_cells;

impl ChatPane {
    /// Rows consumed above the message body: just the single header row. The
    /// old per-agent statusline grid was retired in favour of the whole-pane
    /// summary footer below the composer (see `chatsummary`), so nothing but the
    /// header sits on top now. Tiny panes (`rows < 3`) fall back to the plain
    /// message layout, which draws its own bottom-row prompt instead.
    pub(crate) fn status_rows(&self, _cols: u16, rows: u16) -> u16 {
        if rows < 3 {
            0 // too short — plain message fallback
        } else {
            1 // header only
        }
    }

    /// Back-compat name used by callers that only have `rows`; estimates at a
    /// wide pane. `cells` recomputes with real `cols`.
    pub(crate) fn top_rows(&self, rows: u16) -> u16 {
        self.status_rows(u16::MAX, rows)
    }
}

/// Render `pane` into a `cols` × `rows` grid.
pub(crate) fn cells(pane: &ChatPane, cols: u16, rows: u16) -> Vec<CellView> {
    // One allotment for the whole frame: `grants` is the same source
    // `msg_rows_budget` reads, and it already resolves `top`/`bottom`, so
    // taking them from it keeps this function from computing `status_rows` a
    // second time and drifting from the budget.
    let g = crate::chatplace::grants(pane, cols, rows);
    let top = g.top;
    if top == 0 {
        return layout_cells(
            &pane.messages,
            &pane.input,
            cols,
            rows,
            pane.scroll,
            pane.connected,
        );
    }
    let names = pane.active_names();
    let status = pane.active_status().map(|(label, secs)| {
        // One agent keeps its roster colour; a parallel pack goes accent.
        let color = match names.as_slice() {
            [one] => crate::chatroster::agent_color(one),
            _ => crate::palette::accent(),
        };
        (label, secs, color)
    });
    // Session stats (model, context, tokens) live only in the below-input
    // summary footer (`chatsummary`); the header stays identity + liveness so
    // the same numbers are never repeated in two places.
    let mut cells = crate::chathdr::header_cells(
        cols,
        &pane.channel,
        pane.connected,
        pane.is_busy(),
        status.as_ref().map(|(l, s, c)| (l.as_str(), *s, *c)),
        pane.compact_view,
    );
    // The per-agent statusline grid that used to sit here (rows 1..) was
    // retired: its model/context/token signals are consolidated into the
    // whole-pane summary footer drawn below the composer (see `chatsummary`).
    // Stacked directly above the composer, innermost first: the run's progress
    // bar, then the queued-messages indicator. `chatplace::grants` decides who
    // gets a row — it is the same source `msg_rows_budget` uses, so what is
    // budgeted and what is drawn cannot disagree. A surface it could not seat
    // has a grant of 0 and is SKIPPED below; the anchors used to floor at
    // `.max(top)` instead, which collapsed them onto one row and let
    // last-write-wins hide whichever drew first.
    let bottom = g.bottom;
    let prog_rows = g.prog;
    let queued_rows = g.queued;
    let bar_row = rows.saturating_sub(bottom + prog_rows);
    let indicator_row = rows.saturating_sub(bottom + prog_rows + queued_rows);
    // Decide on what will actually be DRAWN (`visible_messages`, which
    // includes any provisional `streaming` card), not just settled history:
    // a fresh hop's first `Delta` lands before anything ever settles into
    // `pane.messages`, so branching on `messages.is_empty()` alone sent the
    // live text down the onboarding/empty-state branch, which never calls
    // `visible_messages()` — the reply was invisible for exactly as long as
    // it was actually streaming. Same correction `chatplace::placed_lines`
    // already makes.
    let visible = pane.visible_messages();
    if visible.is_empty() {
        // A run can start before any reply lands — the plan-summary message
        // usually exists by fold time, but don't rely on it here. `g.swarm` is
        // 0 when the pane had no row to seat the line in, and then nothing is
        // drawn: the start row used to floor at `.max(top)` instead, which on a
        // saturated budget pushed it onto another surface's row.
        let block_max = rows.saturating_sub(bottom + prog_rows + queued_rows);
        let block_start = block_max.saturating_sub(g.swarm);
        // The empty-state card stops where the live run begins. It used to get
        // `rows - bottom`, which ignores the rows the status line, queued
        // indicator and bar have already claimed — so a run starting on an
        // empty transcript (i.e. every run) interleaved the onboarding text
        // with them. With no live run all three are 0 and this is unchanged.
        cells.extend(crate::chatempty::empty_cells(
            cols,
            block_start,
            top,
            pane.connected,
            &pane.agents,
        ));
        if g.swarm > 0 {
            cells.extend(
                crate::chatswarmview::block_cells(pane, cols, block_start, crate::anim::now_ms())
                    .into_iter()
                    .filter(|c| c.row >= top && c.row < block_max),
            );
        }
    } else {
        let view = crate::chatmsgs::View {
            source: pane.show_source,
            compact: pane.compact_view,
        };
        let msg_rows = crate::chatplace::msg_rows_budget(pane, cols, rows);
        cells.extend(crate::chatmsgs::message_cells(
            &visible,
            cols,
            msg_rows,
            top,
            pane.scroll,
            view,
        ));
        // Scroll affordances sit over the message area's last column/row.
        let total = crate::chatmsgs::card_line_count(&visible, cols, view);
        cells.extend(crate::chatscroll::scrollbar_cells(
            total,
            msg_rows as usize,
            pane.scroll,
            cols.saturating_sub(1),
            top,
        ));
        if pane.scroll > 0 {
            let last = top + msg_rows.saturating_sub(1);
            cells.extend(crate::chatscroll::new_pill_cells(pane.unread, cols, last));
        }
        // The live swarm block sits under the messages, above the composer
        // (and the queued indicator, when showing). Its start row is derived
        // from the BOTTOM of the pane, the same anchor `grants` used to size
        // `msg_rows` from — not `top + msg_rows`, which used to be equivalent
        // only because `tail` didn't exist yet. Now that `grants` also takes
        // a `tail` slice out of the same pool, `top + msg_rows` has `g.tail`
        // baked into it and lands on the SAME row as the tail (both reduce to
        // `rows - bottom - prog - queued - swarm - tail`) — the two surfaces
        // silently overwrote each other whenever both were granted at once
        // (a swarm run streaming while the user has scrolled up, exactly the
        // state the tail exists to serve). Anchoring both independently from
        // `rows` keeps them stacked instead: swarm right above the composer
        // stack, tail directly above swarm.
        let block_max = rows.saturating_sub(bottom + prog_rows + queued_rows);
        let swarm_start = block_max.saturating_sub(g.swarm);
        if g.swarm > 0 {
            cells.extend(
                crate::chatswarmview::block_cells(pane, cols, swarm_start, crate::anim::now_ms())
                    .into_iter()
                    .filter(|c| c.row < block_max),
            );
        }
        // Above the live status line: the streaming overflow tail, when
        // `grants` could seat it (0 rows means it was not budgeted, so it is
        // skipped entirely rather than sharing another surface's row).
        if g.tail > 0 {
            let tail_start = swarm_start.saturating_sub(g.tail);
            cells.extend(crate::chattail::tail_cells(pane, cols, tail_start));
        }
    }
    if queued_rows > 0 {
        cells.extend(crate::chatqueue::indicator_cells(pane, cols, indicator_row));
    }
    if prog_rows > 0 {
        cells.extend(crate::chatprog::bar_cells(
            pane,
            cols,
            bar_row,
            crate::anim::now_ms(),
        ));
    }
    // The composer anchors to the bottom of the pane MINUS the summary footer,
    // so it ends one row up when the footer shows; the summary then occupies the
    // very last row. `g.summary` is the same reservation `grants` budgeted, so
    // nothing above can overdraw the footer.
    let summary_h = g.summary;
    cells.extend(crate::chatinput::composer_cells(
        &pane.input,
        &pane.agents,
        cols,
        rows - summary_h,
    ));
    if summary_h > 0 {
        // The block occupies the last `summary_h` rows, drawn top-down from
        // where the composer ends.
        cells.extend(crate::chatsummary::summary_cells(
            pane,
            cols,
            rows - summary_h,
            summary_h,
        ));
    }
    cells
}

/// The URL a markdown link occupies at `(row, col)` in the message body, if
/// any — `clickopen`'s click hit-test. Re-derives `chatplace::placed_lines` with
/// the same `cols`/`rows` geometry `cells` renders the message area at, so a
/// click can never resolve against stale layout. `col` is a DISPLAY column
/// (what the click's `CellView` carries), so the cell is found via
/// `chatplace::cell_at_col`, which walks the line with the same char-width
/// accounting `line_cells` renders with — not raw `Vec` indexing, which
/// drifts from display columns once a wide (CJK/emoji) or zero-width glyph
/// appears earlier on the line.
pub(crate) fn link_at(pane: &ChatPane, cols: u16, rows: u16, row: u16, col: u16) -> Option<String> {
    crate::chatplace::placed_lines(pane, cols, rows)
        .into_iter()
        .find(|(r, _)| *r == row)
        .and_then(|(_, line)| crate::chatplace::cell_at_col(&line, col).cloned())
        .and_then(|cell: CardCell| cell.link)
        .map(|l| l.to_string())
}

/// The whole fenced code block a click landed in, as text — `None` when the
/// click was not inside one.
///
/// Reading an answer and then USING it are different acts, and the second had
/// no support: a code block could be selected with the mouse like any other
/// text, which is the same amount of work as retyping it for anything longer
/// than a line. Cmd+click already means "act on what is under the cursor"
/// for links; this gives it the other thing worth acting on.
///
/// A block is the run of contiguous rows whose cells carry the code
/// background. The `╭─ lang` and `╰─` chrome deliberately do not (see
/// `chatmd::span_style`), so the run is exactly the code — no fence markers,
/// no language tag, nothing to strip after pasting.
pub(crate) fn code_block_at(pane: &ChatPane, cols: u16, rows: u16, row: u16) -> Option<String> {
    let code_bg = Some(crate::chatink::code_bg());
    let placed = crate::chatplace::placed_lines(pane, cols, rows);
    let is_code = |r: u16| {
        placed
            .iter()
            .find(|(pr, _)| *pr == r)
            .is_some_and(|(_, line)| {
                !line.is_empty() && line.iter().any(|c: &CardCell| c.bg == code_bg)
            })
    };
    if !is_code(row) {
        return None;
    }
    let (mut first, mut last) = (row, row);
    while first > 0 && is_code(first - 1) {
        first -= 1;
    }
    while last + 1 < rows && is_code(last + 1) {
        last += 1;
    }
    let text = |r: u16| -> String {
        placed
            .iter()
            .find(|(pr, _)| *pr == r)
            .map(|(_, line)| line.iter().map(|c| c.c).collect::<String>())
            .unwrap_or_default()
            .trim_end()
            .to_string()
    };
    let body: Vec<String> = (first..=last).map(text).collect();
    // Every code line is indented by one column when placed; strip that back
    // off so what lands on the clipboard is what the agent wrote.
    let indent = body
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    Some(
        body.iter()
            .map(|l| l.chars().skip(indent).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

#[cfg(test)]
#[path = "chatview_tests.rs"]
mod tests;

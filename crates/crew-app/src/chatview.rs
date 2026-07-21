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
    if pane.messages.is_empty() {
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
            &pane.messages,
            cols,
            msg_rows,
            top,
            pane.scroll,
            view,
        ));
        // Scroll affordances sit over the message area's last column/row.
        let total = crate::chatmsgs::card_line_count(&pane.messages, cols, view);
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
        // (and the queued indicator, when showing) — msg_rows_budget already
        // reserved its rows so nothing overlaps in the normal case, but clamp
        // anyway so a saturated row budget can never push block rows onto the
        // composer row (see the empty-branch comment above for why the
        // composer doesn't reliably overdraw them).
        let block_max = rows.saturating_sub(bottom + prog_rows + queued_rows);
        if g.swarm > 0 {
            cells.extend(
                crate::chatswarmview::block_cells(
                    pane,
                    cols,
                    top + msg_rows,
                    crate::anim::now_ms(),
                )
                .into_iter()
                .filter(|c| c.row < block_max),
            );
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

#[cfg(test)]
#[path = "chatview_tests.rs"]
mod tests;

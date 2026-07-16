//! Composes the crew pane's full cell view: the session line (row 0),
//! statusline-style agent rows (one per agent, from `chatchips::layout`),
//! role-styled message cards, and the input composer (bordered fieldset on
//! tall panes, a bare prompt row on short ones) on the bottom rows. Tiny
//! panes fall back to the plain layout.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatbody::CardCell;
use crate::chatlayout::layout_cells;

impl ChatPane {
    /// One `AgentView` per roster agent, snapshotting live state for the grid.
    pub(crate) fn agent_views(&self) -> Vec<crate::chatchips::AgentView> {
        let names = self.active_names();
        let sum_ms: u64 = self.agent_stats.values().map(|(_, ms)| *ms).sum();
        let now = crate::anim::now_ms();
        self.agents
            .iter()
            .map(|a| {
                let active = names.contains(&a.name.as_str());
                let ctx = self.ctx.get(&a.name).copied().unwrap_or(0);
                let ctx_pct = crate::ctxlimit::context_limit(&a.model)
                    .filter(|&l| l > 0)
                    .map(|l| ((ctx * 100) / l).min(100) as u8);
                let agent_ms = self
                    .agent_stats
                    .get(&a.name)
                    .map(|(_, ms)| *ms)
                    .unwrap_or(0);
                let share_pct = (sum_ms > 0).then(|| ((agent_ms * 100) / sum_ms).min(100) as u8);
                // Fresh roster restored from a session has no tok animation
                // entry yet — fall back to the raw ctx value so it doesn't
                // show 0 until the first live update arrives.
                let tok_eased = self.anim.tok(&a.name, now);
                let tok = if tok_eased > 0.0 {
                    tok_eased.round() as u64
                } else {
                    ctx
                };
                crate::chatchips::AgentView {
                    name: a.name.clone(),
                    state: self.agent_state_str(&a.name, active),
                    tok,
                    ctx_pct,
                    share_pct,
                    ctx_frac: self.anim.ctx(&a.name, now),
                    shr_frac: self.anim.shr(&a.name, now),
                    flash_t: self.anim.flash_t(&a.name, now),
                    active,
                }
            })
            .collect()
    }

    /// The state token for an agent chip: live spinner + elapsed while active,
    /// else `·n×` with the reply count, or `idle`.
    fn agent_state_str(&self, name: &str, active: bool) -> String {
        if active {
            if let Some(a) = self.active_agents().iter().find(|a| a.name == name) {
                let f =
                    (a.since.elapsed().as_millis() / 120) as usize % crate::update::SPINNER.len();
                // Space between the spinner "pixels" and the elapsed number.
                return format!(
                    "{} {}s",
                    crate::update::SPINNER[f],
                    a.since.elapsed().as_secs()
                );
            }
        }
        match self.agent_stats.get(name) {
            Some((n, _)) if *n > 0 => format!("\u{00b7} {n}\u{00d7}"),
            _ => "idle".into(),
        }
    }

    /// Total rows consumed above the message body: session line + card grid.
    /// Derives from `chatchips::layout`, the same function the renderer uses,
    /// so the two can never disagree about the drawn extent.
    pub(crate) fn status_rows(&self, cols: u16, rows: u16) -> u16 {
        if rows < 3 {
            return 0; // too short — plain message fallback
        }
        let views = self.agent_views();
        let show_share = views.len() >= 2;
        let avail =
            rows.saturating_sub(1 + crate::chatinput::composer_rows(&self.input, cols, rows));
        match crate::chatchips::layout(&views, cols, avail, show_share) {
            Some(l) => 1 + l.rows,
            None => 1, // session line only
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
    let mut cells = crate::chathdr::header_cells(
        cols,
        &pane.channel,
        pane.connected,
        pane.is_busy(),
        status.as_ref().map(|(l, s, c)| (l.as_str(), *s, *c)),
        pane.tokens,
        pane.compact_view,
    );
    // Zone 2: the statusline-style agent rows (rows 1..1+lay.rows), sized by
    // the same `layout` call `status_rows` used above — so the two always
    // agree on the drawn extent (no overdraw onto the message body).
    let views = pane.agent_views();
    let show_share = views.len() >= 2;
    let avail = rows.saturating_sub(1 + crate::chatinput::composer_rows(&pane.input, cols, rows));
    if let Some(lay) = crate::chatchips::layout(&views, cols, avail, show_share) {
        cells.extend(crate::chatchips::row_cells(
            &views,
            cols,
            1,
            &lay,
            crate::anim::now_ms(),
        ));
    }
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
        cells.extend(crate::chatprog::bar_cells(pane, cols, bar_row));
    }
    cells.extend(crate::chatinput::composer_cells(
        &pane.input,
        &pane.agents,
        cols,
        rows,
    ));
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

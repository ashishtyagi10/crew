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
        let avail =
            rows.saturating_sub(1 + crate::chatinput::composer_rows(&self.input, cols, rows));
        match crate::chatchips::layout(&views, cols, avail) {
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
    let top = pane.status_rows(cols, rows);
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
    // The settled/last turn's duration — the sum of its recorded hop times —
    // folded into the session line as `<N> turn[s] · <D.D>s`.
    let turn_ms: u64 = pane.pulse.hops().iter().map(|(_, ms)| *ms).sum();
    let mut cells = crate::chathdr::header_cells(
        cols,
        &pane.channel,
        pane.connected,
        pane.messages.len(),
        pane.is_busy(),
        status.as_ref().map(|(l, s, c)| (l.as_str(), *s, *c)),
        (pane.tokens, pane.turns),
        turn_ms,
    );
    // Zone 2: the statusline-style agent rows (rows 1..1+lay.rows), sized by
    // the same `layout` call `status_rows` used above — so the two always
    // agree on the drawn extent (no overdraw onto the message body).
    let views = pane.agent_views();
    let avail = rows.saturating_sub(1 + crate::chatinput::composer_rows(&pane.input, cols, rows));
    if let Some(lay) = crate::chatchips::layout(&views, cols, avail) {
        cells.extend(crate::chatchips::row_cells(
            &views,
            cols,
            1,
            &lay,
            crate::anim::now_ms(),
        ));
    }
    let bottom = crate::chatinput::composer_rows(&pane.input, cols, rows);
    // The queued-messages indicator sits directly above the composer,
    // claiming one row from the budget when the queue is non-empty (same
    // pattern as the swarm block claiming rows below the messages).
    let queued_rows = crate::chatqueue::queued_rows(pane);
    // Floored at `top`, same as the swarm block's `block_start` below — on a
    // squeezed pane (a saturated roster eating rows via `status_rows`, plus
    // a small `rows`) the unclamped subtraction can bottom out at or below
    // `top`, letting the indicator overdraw the header/status rows.
    let indicator_row = rows.saturating_sub(bottom + queued_rows).max(top);
    if pane.messages.is_empty() {
        cells.extend(crate::chatempty::empty_cells(
            cols,
            rows - bottom,
            top,
            pane.connected,
            &pane.agents,
        ));
        // A run can start before any reply lands — the plan-summary message
        // usually exists by fold time, but don't rely on it here. The block's
        // rows are clamped to strictly above the composer (and the queued
        // indicator, when showing): a saturated row budget (e.g. an 8-task
        // swarm on a tiny pane) can otherwise push block rows onto the
        // composer row, which doesn't reliably overdraw them (only the
        // prompt glyph/text touch every column). Floored at `top` too — on a
        // very short pane the same saturation can otherwise push the start
        // row above the header/status rows.
        let block_max = rows.saturating_sub(bottom + queued_rows);
        let block_start = block_max
            .saturating_sub(crate::chatswarmview::swarm_rows(pane, rows))
            .max(top);
        cells.extend(
            crate::chatswarmview::block_cells(pane, cols, block_start, crate::anim::now_ms())
                .into_iter()
                .filter(|c| c.row >= top && c.row < block_max),
        );
    } else {
        let msg_rows = crate::chatplace::msg_rows_budget(pane, cols, rows);
        cells.extend(crate::chatmsgs::message_cells(
            &pane.messages,
            cols,
            msg_rows,
            top,
            pane.scroll,
            pane.show_source,
        ));
        // Scroll affordances sit over the message area's last column/row.
        let total = crate::chatmsgs::card_line_count(&pane.messages, cols, pane.show_source);
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
        let block_max = rows.saturating_sub(bottom + queued_rows);
        cells.extend(
            crate::chatswarmview::block_cells(pane, cols, top + msg_rows, crate::anim::now_ms())
                .into_iter()
                .filter(|c| c.row < block_max),
        );
    }
    if queued_rows > 0 {
        cells.extend(crate::chatqueue::indicator_cells(pane, cols, indicator_row));
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

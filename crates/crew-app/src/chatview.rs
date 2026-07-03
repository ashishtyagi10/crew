//! Composes the crew pane's full cell view: status header (row 0), agent
//! roster (row 1 when known), the live activity row (row 2 while agents
//! work), role-styled message cards, and the input composer (affordance bar +
//! prompt) on the bottom rows. Tiny panes fall back to the plain layout.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatlayout::layout_cells;

impl ChatPane {
    /// One `AgentView` per roster agent, snapshotting live state for the grid.
    pub(crate) fn agent_views(&self) -> Vec<crate::chatchips::AgentView> {
        let names = self.active_names();
        let sum_ms: u64 = self.agent_stats.values().map(|(_, ms)| *ms).sum();
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
                crate::chatchips::AgentView {
                    name: a.name.clone(),
                    state: self.agent_state_str(&a.name, active),
                    tok: ctx,
                    ctx_pct,
                    share_pct,
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
                return format!(
                    "{}{}s",
                    crate::update::SPINNER[f],
                    a.since.elapsed().as_secs()
                );
            }
        }
        match self.agent_stats.get(name) {
            Some((n, _)) if *n > 0 => format!("\u{00b7}{n}\u{00d7}"),
            _ => "idle".into(),
        }
    }

    /// Total rows consumed above the message body: session line + card grid +
    /// the turn waterfall (only once a turn has run AND the pane is wide enough
    /// for `waterfall_cells` to draw — it needs `cols >= 30`, so the count must
    /// match or the body sizing drifts). Derives from `chatchips::layout`, the
    /// same function the renderer uses, so the two can never disagree about
    /// the drawn extent.
    pub(crate) fn status_rows(&self, cols: u16, rows: u16) -> u16 {
        if rows < 3 {
            return 0; // too short — plain message fallback
        }
        let views = self.agent_views();
        let wf = u16::from(!self.pulse.hops().is_empty() && cols >= 30);
        let avail = rows.saturating_sub(2).saturating_sub(1 + wf);
        match crate::chatchips::layout(&views, cols, avail) {
            Some(l) => 1 + l.rows + wf,
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
    let mut cells = crate::chathdr::header_cells(
        cols,
        &pane.channel,
        pane.connected,
        pane.messages.len(),
        pane.is_busy(),
        status.as_ref().map(|(l, s, c)| (l.as_str(), *s, *c)),
        (pane.tokens, pane.turns),
    );
    // Zone 2: the statusline-style agent rows (rows 1..1+lay.rows), sized by
    // the same `layout` call `status_rows` used above — so the two always
    // agree on the drawn extent (no overdraw onto the message body).
    let views = pane.agent_views();
    let wf_possible = !pane.pulse.hops().is_empty() && cols >= 30;
    let avail = rows
        .saturating_sub(2)
        .saturating_sub(1 + u16::from(wf_possible));
    if let Some(lay) = crate::chatchips::layout(&views, cols, avail) {
        cells.extend(crate::chatchips::row_cells(&views, cols, 1, &lay));
        // Zone 3: the turn waterfall below the grid, once a turn ran (and only
        // when wide enough to draw — matches status_rows' cols>=30 gate).
        // `live` is the newest thinking agent's still-growing segment.
        if wf_possible {
            let live = pane
                .active_agents()
                .last()
                .map(|a| (a.name.as_str(), a.since.elapsed().as_millis() as u64));
            cells.extend(crate::chatpulse::waterfall_cells(
                cols,
                1 + lay.rows,
                pane.pulse.hops(),
                live,
            ));
        }
    }
    let bottom = crate::chatinput::composer_rows(rows);
    if pane.messages.is_empty() {
        cells.extend(crate::chatempty::empty_cells(
            cols,
            rows - bottom,
            top,
            pane.connected,
            &pane.agents,
        ));
    } else {
        let msg_rows = rows.saturating_sub(top + bottom);
        cells.extend(crate::chatmsgs::message_cells(
            &pane.messages,
            cols,
            msg_rows,
            top,
            pane.scroll,
        ));
        // Scroll affordances sit over the message area's last column/row.
        let total = crate::chatmsgs::card_line_count(&pane.messages, cols);
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
    }
    cells.extend(crate::chatinput::composer_cells(
        &pane.input,
        &pane.agents,
        cols,
        rows,
    ));
    cells
}

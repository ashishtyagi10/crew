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
                    model: a.model.clone(),
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

    /// Total rows consumed above the message body: session line + chip grid +
    /// the turn waterfall (only once a turn has run AND the pane is wide enough
    /// for `waterfall_cells` to draw — it needs `cols >= 30`, so the count must
    /// match or the body sizing drifts). Replaces the old header+lanes /
    /// header+roster+activity accounting.
    pub(crate) fn status_rows(&self, cols: u16, rows: u16) -> u16 {
        if rows < 3 {
            return 0; // too short — plain message fallback
        }
        let grid = crate::chatchips::grid_rows(&self.agent_views(), cols);
        let waterfall = u16::from(!self.pulse.hops().is_empty() && cols >= 30);
        (1 + grid + waterfall).min(rows.saturating_sub(2))
    }

    /// Back-compat name used by callers that only have `rows`; estimates at a
    /// wide pane. `cells` recomputes with real `cols`.
    pub(crate) fn top_rows(&self, rows: u16) -> u16 {
        self.status_rows(u16::MAX, rows)
    }

    /// Pulse lanes to draw (0 = pulse hidden, keep the legacy rows).
    pub(crate) fn pulse_lanes(&self, rows: u16) -> u16 {
        crate::chatpulse::pulse_lanes(self.agents.len(), rows, self.engaged())
    }
}

/// The pulse block under the header: one lane per agent (rows 1..=lanes) and
/// the turn waterfall (row lanes+1). Lanes share a sparkline scale so hop
/// heights compare across agents; the share bars split total reply time.
fn pulse_block(pane: &ChatPane, cols: u16, lanes: u16) -> Vec<CellView> {
    let shown = &pane.agents[..(lanes as usize).min(pane.agents.len())];
    let name_w = shown
        .iter()
        .map(|a| a.name.chars().count())
        .max()
        .unwrap_or(0);
    let scale = shown
        .iter()
        .filter_map(|a| pane.pulse.hist(&a.name))
        .map(|h| h.peak(crate::chatpulse::SPARK_W as usize))
        .max()
        .unwrap_or(0)
        .max(1);
    let total_ms: u64 = pane.agent_stats.values().map(|(_, ms)| ms).sum();
    let mut cells = Vec::new();
    for (i, a) in shown.iter().enumerate() {
        let active = pane.active_agents().iter().find(|x| x.name == a.name);
        cells.extend(crate::chatpulse::lane_cells(
            cols,
            1 + i as u16,
            a,
            active,
            pane.pulse.hist(&a.name),
            &pane.agent_stats,
            total_ms,
            scale,
            name_w,
            pane.ctx.get(&a.name).copied(),
            crate::ctxlimit::context_limit(&a.model),
        ));
    }
    // The newest thinking agent's segment grows live on the waterfall.
    let live = pane
        .active_agents()
        .last()
        .map(|a| (a.name.as_str(), a.since.elapsed().as_millis() as u64));
    cells.extend(crate::chatpulse::waterfall_cells(
        cols,
        1 + lanes,
        pane.pulse.hops(),
        live,
    ));
    cells
}

/// Render `pane` into a `cols` × `rows` grid.
pub(crate) fn cells(pane: &ChatPane, cols: u16, rows: u16) -> Vec<CellView> {
    let top = pane.top_rows(rows);
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
    let lanes = pane.pulse_lanes(rows);
    if lanes > 0 {
        cells.extend(pulse_block(pane, cols, lanes));
    } else {
        if top > 1 {
            cells.extend(crate::chatroster::roster_cells(
                cols,
                1,
                &pane.agents,
                &names,
                &pane.agent_stats,
            ));
        }
        // While agents work, row 2 shows who is doing what for whom, live.
        if top > 2 {
            cells.extend(crate::chatflow::activity_cells(
                cols,
                2,
                pane.active_agents(),
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

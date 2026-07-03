//! The crew pane's pulse tracking: per-agent hop timings observed from
//! `Activity`/reply events, feeding the turn waterfall row — the relay's hops
//! as proportional colored segments below the chip grid.
use std::collections::HashMap;

use crew_render::CellView;

use crate::chatroster::agent_color;
use crate::spark::History;

/// Hop-duration samples kept per agent (sparkline window and then some).
const HIST_CAP: usize = 32;

/// Per-agent hop timings observed live from `Activity`/reply events: the
/// per-agent hop-duration history and the waterfall's hops for the in-flight
/// (or last completed) turn.
pub(crate) struct Pulse {
    hist: HashMap<String, History>,
    hops: Vec<(String, u64)>,
    turn_done: bool,
}

impl Pulse {
    pub(crate) fn new() -> Self {
        Pulse {
            hist: HashMap::new(),
            hops: Vec::new(),
            turn_done: false,
        }
    }

    /// A hop is starting: a settled turn's waterfall resets so the new turn
    /// draws from a clean row; mid-turn this is a no-op.
    pub(crate) fn begin_hop(&mut self) {
        if self.turn_done {
            self.hops.clear();
            self.turn_done = false;
        }
    }

    /// A hop finished after `ms`: append to the waterfall and the agent's
    /// sparkline history.
    pub(crate) fn record_hop(&mut self, agent: &str, ms: u64) {
        self.hist
            .entry(agent.to_string())
            .or_insert_with(|| History::new(HIST_CAP))
            .push(ms);
        self.hops.push((agent.to_string(), ms));
    }

    /// The turn is over: freeze the waterfall as the settled record until the
    /// next turn's first hop.
    pub(crate) fn end_turn(&mut self) {
        self.turn_done = true;
    }

    /// Per-agent hop-duration history — kept for its own tested contract
    /// (`record_hop` accumulation, capped window, surviving turn resets)
    /// even though the lane sparkline that used to render it is gone; a
    /// future per-agent trend view is the natural consumer.
    #[allow(dead_code)]
    pub(crate) fn hist(&self, agent: &str) -> Option<&History> {
        self.hist.get(agent)
    }

    pub(crate) fn hops(&self) -> &[(String, u64)] {
        &self.hops
    }
}

/// Append `s` at `(row, col..)` in `fg`, clipped to `max_col`; returns the next column.
fn push(
    cells: &mut Vec<CellView>,
    row: u16,
    col: u16,
    max_col: u16,
    s: &str,
    fg: (u8, u8, u8),
    bold: bool,
) -> u16 {
    let bg = crew_theme::theme().page_bg;
    crate::chatwidth::place_row(col, max_col, s.chars().map(|c| (c, fg)), |x, c, fg| {
        cells.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg,
            bold,
            italic: false,
        });
    })
}

/// The turn waterfall: `turn ▶ ████ ██ █ 12.4s` — one segment per hop,
/// width proportional to its duration, in the agent's colour, separated by
/// 1-cell page gaps. `live` appends the thinking agent's still-growing
/// segment; the trailing label totals the turn so far.
pub(crate) fn waterfall_cells(
    cols: u16,
    row: u16,
    hops: &[(String, u64)],
    live: Option<(&str, u64)>,
) -> Vec<CellView> {
    let mut segs: Vec<(&str, u64)> = hops.iter().map(|(n, ms)| (n.as_str(), *ms)).collect();
    if let Some((name, ms)) = live {
        segs.push((name, ms));
    }
    if segs.is_empty() || cols < 30 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let mut cells = Vec::new();
    let label = if live.is_some() {
        "turn \u{25b6} "
    } else {
        "turn "
    };
    let x0 = push(&mut cells, row, 0, cols, label, t.text_muted, false);

    let total: u64 = segs.iter().map(|(_, ms)| ms).sum::<u64>().max(1);
    let tail = format!(" {:.1}s", total as f64 / 1000.0);
    let area = cols
        .saturating_sub(x0)
        .saturating_sub(tail.chars().count() as u16);
    let gaps = segs.len() as u16 - 1;
    let net = area.saturating_sub(gaps);
    if net < segs.len() as u16 {
        return Vec::new(); // too narrow for one cell per hop
    }
    let mut x = x0;
    for (i, (name, ms)) in segs.iter().enumerate() {
        if i > 0 {
            x += 1; // surface gap between segments
        }
        let w = ((*ms as f64 / total as f64) * net as f64).round().max(1.0) as u16;
        let w = w.min(x0 + area - x); // clip the last segment to the area
        let color = agent_color(name);
        for k in 0..w {
            cells.push(CellView {
                col: x + k,
                row,
                c: '\u{2588}', // █
                fg: color,
                bg: t.page_bg,
                bold: false,
                italic: false,
            });
        }
        x += w;
    }
    push(&mut cells, row, x, cols, &tail, t.text_muted, false);
    cells
}

#[cfg(test)]
#[path = "chatpulse_tests.rs"]
mod tests;

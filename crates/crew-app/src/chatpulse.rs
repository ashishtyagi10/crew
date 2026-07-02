//! The crew pane's "pulse" dashboard: one lane per agent (identity, live
//! state + elapsed timer, hop-duration sparkline, time-share bar) plus a turn
//! waterfall row — the relay's hops as proportional colored segments. Lanes
//! replace the roster/activity rows on tall panes once a turn has run, so the
//! pane reads like an agent monitor: fixed lanes, stable per-agent colors,
//! marks in color and text in ink.
use std::collections::HashMap;

use crew_plugin::AgentInfo;
use crew_render::CellView;

use crate::chatflow::ActiveAgent;
use crate::chatroster::{agent_color, chip_stat};
use crate::spark::{line_cells, History};

/// Hop-duration samples kept per agent (lane sparkline window and then some).
const HIST_CAP: usize = 32;
/// Sparkline width in cells; lanes share one scale across this window.
pub(crate) const SPARK_W: u16 = 16;
/// Share bar width in cells.
const BAR_W: u16 = 10;
/// Lanes shown at most (default crew is 3; beyond 6 the block stops scaling).
const MAX_LANES: usize = 6;
/// Milliseconds per spinner frame (matches the activity row's cadence).
const FRAME_MS: u128 = 120;

/// Per-agent hop timings observed live from `Activity`/reply events: the lane
/// sparklines' history and the waterfall's hops for the in-flight (or last
/// completed) turn.
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

    pub(crate) fn hist(&self, agent: &str) -> Option<&History> {
        self.hist.get(agent)
    }

    pub(crate) fn hops(&self) -> &[(String, u64)] {
        &self.hops
    }

    /// Nothing recorded yet — the pane keeps its legacy roster rows.
    pub(crate) fn is_empty(&self) -> bool {
        self.hops.is_empty() && self.hist.is_empty()
    }
}

/// Lanes the pulse block shows: one per agent (capped) on panes tall enough,
/// once the crew is engaged (a turn ran or agents are thinking). 0 = keep the
/// legacy roster/activity rows.
pub(crate) fn pulse_lanes(agents: usize, rows: u16, engaged: bool) -> u16 {
    if !engaged || agents == 0 || rows < 14 {
        return 0;
    }
    agents.min(MAX_LANES) as u16
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

/// One agent's lane: `▸ name ⠹ 4s  ▂▃▅▂▁▄▂▃  ██████░░░░ 62%`.
///
/// Marker + name in the agent's stable colour (bold + `▸` while thinking),
/// then live spinner + elapsed seconds (thinking) or the dimmed `·n× avg`
/// stat (idle). The right side carries the hop-duration sparkline (all lanes
/// share `scale`) and the agent's share of total reply time as a gauge bar.
/// Regions drop out progressively as the pane narrows.
#[allow(clippy::too_many_arguments)]
pub(crate) fn lane_cells(
    cols: u16,
    row: u16,
    agent: &AgentInfo,
    active: Option<&ActiveAgent>,
    hist: Option<&History>,
    stats: &HashMap<String, (u32, u64)>,
    total_ms: u64,
    scale: u64,
    name_w: usize,
) -> Vec<CellView> {
    let t = crew_theme::theme();
    let color = agent_color(&agent.name);
    let mut cells = Vec::new();

    // Right-side chart regions, laid out from the right edge inward.
    let bar_region = BAR_W + 5; // bar + " NN%"
    let has_bar = cols >= 62 && total_ms > 0;
    let has_spark = cols >= 46;
    let bar_col = cols.saturating_sub(bar_region);
    let spark_col = if has_bar {
        bar_col.saturating_sub(SPARK_W + 1)
    } else {
        cols.saturating_sub(SPARK_W)
    };
    // Left text clips before the first chart region.
    let text_max = match (has_spark, has_bar) {
        (true, _) => spark_col.saturating_sub(1),
        (false, true) => bar_col.saturating_sub(1),
        (false, false) => cols,
    };

    let is_active = active.is_some();
    let marker = if is_active { "\u{25b8} " } else { "\u{25aa} " }; // ▸ / ▪
    let mut x = push(&mut cells, row, 0, text_max, marker, color, is_active);
    let name = format!("{:<name_w$}", agent.name);
    x = push(&mut cells, row, x, text_max, &name, color, is_active);
    x = push(&mut cells, row, x, text_max, " ", t.text_muted, false);
    match active {
        Some(a) => {
            // Live: spinner + elapsed ride the busy-animation redraw frames.
            let frame = ((a.since.elapsed().as_millis() / FRAME_MS) as usize)
                % crate::update::SPINNER.len();
            let live = format!(
                "{} {}s",
                crate::update::SPINNER[frame],
                a.since.elapsed().as_secs()
            );
            push(
                &mut cells,
                row,
                x,
                text_max,
                &live,
                crate::palette::accent(),
                false,
            );
        }
        None => {
            let stat = chip_stat(stats, &agent.name);
            push(&mut cells, row, x, text_max, &stat, t.text_muted, false);
        }
    }

    if has_spark {
        if let Some(h) = hist {
            cells.extend(line_cells(h, SPARK_W, spark_col, row, scale, color));
        }
    }
    if has_bar {
        let ms = stats.get(&agent.name).map(|(_, ms)| *ms).unwrap_or(0);
        let frac = ms as f64 / total_ms as f64;
        let fill = (frac * BAR_W as f64).round() as u16;
        for i in 0..BAR_W {
            let (c, fg) = if i < fill {
                ('\u{2588}', color) // █
            } else {
                ('\u{2591}', t.border_normal) // ░
            };
            cells.push(CellView {
                col: bar_col + i,
                row,
                c,
                fg,
                bg: t.page_bg,
                bold: false,
                italic: false,
            });
        }
        let pct = format!(" {:>3.0}%", frac * 100.0);
        push(
            &mut cells,
            row,
            bar_col + BAR_W,
            cols,
            &pct,
            t.text_muted,
            false,
        );
    }
    cells
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

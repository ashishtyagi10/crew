use std::time::{SystemTime, UNIX_EPOCH};

use crew_render::CellView;

use crate::clock;
use crate::gauges::render_stats;
use crate::stats::SysSampler;

/// The docked sidebar: a live clock card stacked above the system-stats card.
pub struct StatsPane {
    sampler: SysSampler,
    /// Last wall-clock second shown, so the clock repaints once per second.
    last_sec: u64,
}

impl StatsPane {
    pub fn new() -> Self {
        Self {
            sampler: SysSampler::new(),
            last_sec: 0,
        }
    }

    /// Returns true when the sidebar should repaint — either fresh stats
    /// (~1s throttle) or a new wall-clock second for the clock.
    pub fn refresh(&mut self) -> bool {
        let stats_changed = self.sampler.refresh();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let clock_changed = now != self.last_sec;
        self.last_sec = now;
        stats_changed || clock_changed
    }

    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        let (time, date) = clock::now_strings();
        let mut out = clock::clock_cells(&time, &date, cols);
        if rows > clock::CLOCK_H {
            for mut c in render_stats(self.sampler.stats(), cols, rows - clock::CLOCK_H) {
                c.row += clock::CLOCK_H;
                out.push(c);
            }
        }
        out
    }
}

impl Default for StatsPane {
    fn default() -> Self {
        Self::new()
    }
}

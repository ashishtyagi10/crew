use crew_render::CellView;

use crate::gauges::render_stats;
use crate::stats::SysSampler;

/// The system-stats content of the docked sidebar. Renders to cells via
/// [`render_stats`]; refreshes its sampler on a ~1s throttle.
pub struct StatsPane {
    sampler: SysSampler,
}

impl StatsPane {
    pub fn new() -> Self {
        Self {
            sampler: SysSampler::new(),
        }
    }

    pub fn refresh(&mut self) -> bool {
        self.sampler.refresh()
    }

    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        render_stats(self.sampler.stats(), cols, rows)
    }
}

impl Default for StatsPane {
    fn default() -> Self {
        Self::new()
    }
}

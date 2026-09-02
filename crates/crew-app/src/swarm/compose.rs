//! One frame of a running swarm, assembled from a snapshot: the task list on
//! the left, the bars on the right, the notice on the last row.
//!
//! [`SwarmPane`](crate::swarmpane::SwarmPane) holds the live graph, fleet and
//! timeline; the shot harness holds fixtures. Both hand a [`Run`] here, so the
//! picture a test looks at is the one the pane draws — not a re-assembly of
//! the same parts that could drift from it.
use crew_hive::{Fleet, TaskGraph};
use crew_render::{CellView, Paint};

use super::timeline::Timeline;
use super::view::{
    cancelled_notice, state_color, swarm_cells, timeline_cells, timeline_cols, timeline_paint,
};

/// A running swarm as the view needs it.
pub struct Run<'a> {
    pub graph: &'a TaskGraph,
    pub fleet: &'a Fleet,
    pub timeline: &'a Timeline,
    /// The budget governor (or the user) stopped it.
    pub cancelled: bool,
}

impl Run<'_> {
    /// The timeline's axis at `now` — `None` until a task has started.
    fn axis(&self, now: u64) -> Option<(u64, u64)> {
        (!self.timeline.is_empty()).then(|| self.timeline.axis(now))
    }

    /// Rows the list may use: all of them, or all but the notice's.
    fn list_rows(&self, rows: u16) -> u16 {
        if self.cancelled {
            rows.saturating_sub(1)
        } else {
            rows
        }
    }

    /// The text: HUD, task rows, axis labels, and the cancelled notice.
    pub fn cells(&self, cols: u16, rows: u16, now: u64) -> Vec<CellView> {
        if cols == 0 || rows == 0 {
            return Vec::new();
        }
        // The task list gives up the columns the timeline draws in.
        let list_w = cols - timeline_cols(cols);
        let mut cells = swarm_cells(self.graph, self.fleet, list_w, self.list_rows(rows));
        cells.extend(timeline_cells(cols, rows, self.axis(now)));
        if self.cancelled {
            cells.extend(cancelled_notice(cols, rows));
        }
        cells
    }

    /// The task bars, drawn (see [`crate::plot::gantt`]). Empty until a task
    /// has started, and on a pane too narrow for both halves.
    pub fn paint(&self, cols: u16, rows: u16, aspect: f32, now: u64) -> Vec<Paint> {
        let Some(axis) = self.axis(now) else {
            return Vec::new();
        };
        if timeline_cols(cols) == 0 || rows < 3 {
            return Vec::new();
        }
        let ids: Vec<_> = self.graph.tasks().iter().map(|t| t.id).collect();
        let mut spans = self.timeline.spans_for(&ids, self.fleet, now, state_color);
        // One lane per NAMED task: a bar beside `… +N more` would belong to a
        // task the row does not name.
        spans.truncate(super::rows::shown(ids.len(), self.list_rows(rows)));
        timeline_paint(&spans, cols, rows, aspect, axis, now)
    }
}

#[cfg(test)]
#[path = "compose_tests.rs"]
mod tests;

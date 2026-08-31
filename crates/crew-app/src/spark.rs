//! Rolling history for the sidebar's charts: a [`History`] keeps the last N
//! samples, and the widgets in [`crate::plot`] draw them.
//!
//! It used to render as well — one vertical block glyph (`▁`–`█`) per column,
//! eight height levels, one sample per cell. Every caller has since moved to a
//! drawn chart (area, twin, ring), and the block ramp is gone with them.
//! Charts still move on the sidebar's existing ~1 Hz refresh, so they cost
//! nothing beyond the repaint that already happens each second.
use std::collections::VecDeque;

/// Fixed-capacity ring of recent samples (oldest at the front, newest at back).
pub struct History {
    cap: usize,
    data: VecDeque<u64>,
}

impl History {
    pub fn new(cap: usize) -> Self {
        let cap = cap.max(1);
        Self {
            cap,
            data: VecDeque::with_capacity(cap),
        }
    }

    /// Append a sample, dropping the oldest once capacity is reached.
    pub fn push(&mut self, v: u64) {
        if self.data.len() == self.cap {
            self.data.pop_front();
        }
        self.data.push_back(v);
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The most recent `width` samples (or fewer), oldest first — what fills the
    /// chart left→right so the newest reading sits at the right edge.
    pub fn tail(&self, width: usize) -> Vec<u64> {
        let start = self.data.len().saturating_sub(width);
        self.data.iter().skip(start).copied().collect()
    }

    /// The largest of the most recent `width` samples (0 when empty) — lets
    /// several charts share one scale so their heights compare.
    pub fn peak(&self, width: usize) -> u64 {
        self.tail(width).into_iter().max().unwrap_or(0)
    }
}

#[cfg(test)]
#[path = "spark_tests.rs"]
mod tests;

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crew_render::{CellView, Paint};

use crate::clock;
use crate::gauges::render_stats;
use crate::git::{self, GitWatch};
use crate::host;
use crate::load;
use crate::navlog;
use crate::net;
use crate::panelist::{self, PaneRow};
use crate::stats::SysSampler;

/// Rows the SYSTEM section occupies (rule + 3 gauges + the 2-row CPU area
/// chart + gap). The chart got its second row when it stopped being a line of
/// block glyphs: a curve with a filled body needs the height to say anything
/// the gauge above it does not already say.
const SYS_BLOCK: u16 = 7;
/// Rows the CPU chart occupies, and where it starts inside the SYSTEM block.
const CHART_ROWS: u16 = 2;
const CHART_OFF: u16 = 4;
/// Rows the LOAD section occupies (rule + 1 line + a one-row gap below it).
const LOAD_BLOCK: u16 = 3;
/// Rows a section with a rule + 2 content rows + one-row gap occupies (HOST, NET, GIT).
const CARD_BLOCK: u16 = 4;

/// The docked sidebar: a live clock card stacked above the system-stats card.
pub struct StatsPane {
    sampler: SysSampler,
    /// Last wall-clock second shown, so the clock repaints once per second.
    last_sec: u64,
    /// Git status for the working directory, queried off the main thread.
    git: GitWatch,
    cpu_hist: crate::spark::History, // recent CPU %, drawn as a moving sparkline
    /// Busy-pane count per second — the crew-pulse chart under PANES.
    pulse_hist: crate::spark::History,
}

impl StatsPane {
    pub fn new() -> Self {
        Self {
            sampler: SysSampler::new(),
            last_sec: 0,
            git: GitWatch::default(),
            cpu_hist: crate::spark::History::new(64),
            pulse_hist: crate::spark::History::new(64),
        }
    }

    /// Returns true when the sidebar should repaint — fresh stats (~1s throttle),
    /// a new wall-clock second for the clock, or changed git status for `cwd`.
    /// The watched repo's current branch, when `cwd` is a git repo — read by
    /// `poll_panes` to mirror into each smith pane's summary footer.
    pub fn branch(&self) -> Option<&str> {
        self.git.info().map(|g| g.branch.as_str())
    }

    pub fn refresh(&mut self, cwd: &Path, busy_now: u64) -> bool {
        let stats_changed = self.sampler.refresh();
        if stats_changed {
            // One reading per sample → the sparklines scroll ~1 Hz.
            let cpu = (self.sampler.stats().cpu.clamp(0.0, 1.0) * 100.0).round() as u64;
            self.cpu_hist.push(cpu);
            self.pulse_hist.push(busy_now);
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let clock_changed = now != self.last_sec;
        self.last_sec = now;
        // Off-the-main-thread git status: never blocks the event loop.
        let git_changed = self.git.poll(cwd, now);
        stats_changed || clock_changed || git_changed
    }

    /// Content row where the LOG section's rule sits — everything above it is
    /// the fixed stat cards. The hit path for scrolling the log reads it, so
    /// draw and wheel agree about which rows are the log's.
    pub fn log_top(&self) -> u16 {
        let stats = clock::CLOCK_H + SYS_BLOCK + LOAD_BLOCK + CARD_BLOCK + CARD_BLOCK;
        stats
            + if self.git.info().is_some() {
                CARD_BLOCK
            } else {
                0
            }
    }

    /// The cell-row where the PANES section header sits — used to hit-test
    /// clicks on the pane list. Must track the section offsets in `cells`,
    /// including the conditional GIT and LOG blocks (`log_len` = buffered
    /// entries, so the caller passes `app.log.len()`).
    pub fn panes_top(&self, log_len: usize) -> u16 {
        self.log_top() + navlog::log_block(log_len)
    }

    /// The sidebar's drawn layer: sub-cell [`Paint`] for the charts, in the
    /// card interior's own cell coordinates. `aspect` is the frame's
    /// `cell_h / cell_w`, without which a circle would come out an ellipse and
    /// a chart's proportions would change with the font.
    ///
    /// Kept beside [`Self::cells`] rather than folded into it because the two
    /// layers are drawn by different passes; both read the same section
    /// offsets, which is what keeps a chart under the section it belongs to.
    pub fn chart_paint(
        &self,
        cols: u16,
        rows: u16,
        aspect: f32,
        panes: &[PaneRow],
        log_len: usize,
    ) -> Vec<Paint> {
        let mut out = self.cpu_chart(cols, rows, aspect);
        // The crew donut, under the PANES header — the same offset
        // `cells` lays the list out from, so ring and rows cannot drift apart.
        let panes_off = self.panes_top(log_len);
        if !panes.is_empty() && rows > panes_off + 1 + crate::crewpie::ROWS {
            out.extend(crate::crewpie::paint(
                &crate::crewpie::mix(panes),
                cols,
                panes_off + 1,
                aspect,
                &self.pulse_hist,
            ));
        }
        out
    }

    /// The SYSTEM section's CPU history chart.
    fn cpu_chart(&self, cols: u16, rows: u16, aspect: f32) -> Vec<Paint> {
        let row0 = clock::CLOCK_H + CHART_OFF;
        // Indented under the section legend like the gauges above it, one
        // column of air kept on the right.
        let (col0, width) = (3u16, cols.saturating_sub(4));
        if width == 0 || rows < row0 + CHART_ROWS || self.cpu_hist.is_empty() {
            return Vec::new();
        }
        let samples: Vec<f32> = self
            .cpu_hist
            .tail(width as usize * 2)
            .into_iter()
            .map(|v| (v as f32 / 100.0).clamp(0.0, 1.0))
            .collect();
        let mut c = crate::plot::Canvas::new(width, CHART_ROWS, aspect);
        let (w, h) = c.size();
        crate::plot::area::draw(&mut c, (0.0, 0.0, w, h), &samples, crate::palette::accent());
        c.paint()
            .into_iter()
            .map(|p| p.shifted(f32::from(col0), f32::from(row0)))
            .collect()
    }

    pub fn cells(
        &self,
        cols: u16,
        rows: u16,
        panes: &[PaneRow],
        log: &[crate::applog::LogEntry],
        // How far back the LOG is scrolled — 0 follows the newest line.
        log_back: usize,
    ) -> Vec<CellView> {
        let (time, date) = clock::now_strings();
        let mut out = clock::clock_cells(&time, &date, cols);

        let sys_off = clock::CLOCK_H;
        if rows > sys_off {
            for mut c in render_stats(self.sampler.stats(), cols, rows - sys_off) {
                c.row += sys_off;
                out.push(c);
            }
        }
        // The CPU history chart lives below the three gauges. It is *drawn*,
        // not spelled — see `chart_paint`; nothing is emitted here.

        let load_off = clock::CLOCK_H + SYS_BLOCK;
        if rows > load_off + 1 {
            let (one, five, fifteen) = load::load_avg();
            for mut c in load::load_cells(one, five, fifteen, load::cores(), cols) {
                c.row += load_off;
                out.push(c);
            }
        }

        let host_off = load_off + LOAD_BLOCK;
        if rows > host_off + 3 {
            let (name, uptime) = host::host_strings();
            for mut c in host::host_cells(&name, &uptime, cols) {
                c.row += host_off;
                out.push(c);
            }
        }

        let net_off = host_off + CARD_BLOCK;
        if rows > net_off + 3 {
            let s = self.sampler.stats();
            for mut c in net::net_cells(s.net_rx, s.net_tx, self.sampler.net_hist(), cols) {
                c.row += net_off;
                out.push(c);
            }
        }

        let git_off = net_off + CARD_BLOCK;
        let mut next = git_off;
        if let Some(info) = self.git.info() {
            if rows > git_off + 3 {
                for mut c in git::git_cells(info, cols) {
                    c.row += git_off;
                    out.push(c);
                }
            }
            next = git_off + CARD_BLOCK; // only reserve the GIT block when shown
        }

        // LIVE LOG: recent status messages in their own section, above the panes.
        let log_h = navlog::log_block(log.len());
        if log_h > 0 && rows > next + 1 {
            let fit = ((rows - next - 1) as usize).min(navlog::LOG_LINES);
            for mut c in navlog::log_cells(log, cols, fit, log_back) {
                c.row += next;
                out.push(c);
            }
        }
        let panes_off = next + log_h;

        // PANES list fills the remaining height below the LOG section (header
        // + pulse chart + one row per pane).
        let list_off = 1 + crate::crewpie::ROWS;
        if !panes.is_empty() && rows > panes_off + list_off {
            let limit = (rows - panes_off - list_off) as usize;
            let spin = crate::update::SPINNER
                [(crate::anim::now_ms() / 100) as usize % crate::update::SPINNER.len()];
            for mut c in panelist::pane_cells(panes, cols, limit, spin) {
                c.row += panes_off;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panes_top_accounts_for_git_and_log() {
        let mut s = StatsPane::new();
        // clock(4) + system(7) + load(3) + host(4) + net(4) = 22
        assert_eq!(s.panes_top(0), 22);
        s.git.set_info(Some(git::GitInfo {
            branch: "main".into(),
            changed: 0,
            ahead: 0,
            behind: 0,
        }));
        assert_eq!(s.panes_top(0), 26); // + git(4)
                                        // a non-empty log adds its block: rule + min(n, LOG_LINES) + gap.
        assert_eq!(s.panes_top(2), 26 + 4); // 2 entries -> 2 + 2
        assert_eq!(s.panes_top(99), 26 + navlog::LOG_LINES as u16 + 2); // capped
    }
}

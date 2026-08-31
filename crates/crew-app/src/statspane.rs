use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crew_render::{CellView, Paint};

use crate::clock;
use crate::gauges::render_stats;
use crate::git::{self, GitWatch};
use crate::host;
use crate::load;
use crate::navlayout::{self, NavLayout, CHART_OFF, CHART_ROWS};
use crate::navlog;
use crate::net;
use crate::panelist::{self, PaneRow};
use crate::stats::SysSampler;

/// The smallest peak the CPU chart's axis scales to, in percent — below it the
/// trace draws small rather than being magnified into a busy-looking minute.
const CHART_FLOOR: u64 = 25;

/// The docked sidebar: a live clock card stacked above the system-stats card.
pub struct StatsPane {
    sampler: SysSampler,
    /// Last wall-clock second shown, so the clock repaints once per second.
    last_sec: u64,
    /// Git status for the working directory, queried off the main thread.
    git: GitWatch,
    cpu_hist: crate::spark::History, // recent CPU %, drawn as a moving sparkline
}

impl StatsPane {
    pub fn new() -> Self {
        Self {
            sampler: SysSampler::new(),
            last_sec: 0,
            git: GitWatch::default(),
            cpu_hist: crate::spark::History::new(64),
        }
    }

    /// Pin a git status, so an off-screen shot of the nav includes the GIT
    /// section — it is polled off the main thread and has not answered by the
    /// time a shot is taken, which made it the one section never in frame.
    #[cfg(test)]
    pub fn set_git(&mut self, info: Option<git::GitInfo>) {
        self.git.set_info(info);
    }

    /// Push a minute of synthetic CPU history, so an off-screen shot of the
    /// nav shows the trace a running machine would have rather than the
    /// single sample one `refresh` leaves behind.
    #[cfg(test)]
    pub fn seed_history(&mut self, cpu: &[u64]) {
        for &v in cpu {
            self.cpu_hist.push(v);
        }
    }

    /// Returns true when the sidebar should repaint — fresh stats (~1s throttle),
    /// a new wall-clock second for the clock, or changed git status for `cwd`.
    /// The watched repo's current branch, when `cwd` is a git repo — read by
    /// `poll_panes` to mirror into each smith pane's summary footer.
    pub fn branch(&self) -> Option<&str> {
        self.git.info().map(|g| g.branch.as_str())
    }

    pub fn refresh(&mut self, cwd: &Path) -> bool {
        let stats_changed = self.sampler.refresh();
        if stats_changed {
            // One reading per sample → the sparkline scrolls ~1 Hz.
            let cpu = (self.sampler.stats().cpu.clamp(0.0, 1.0) * 100.0).round() as u64;
            self.cpu_hist.push(cpu);
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

    /// How this nav divides `rows` content rows right now — the one derivation
    /// the draw, the paint layer and both hit paths read, so a click can never
    /// land on a row the frame put something else on. See [`crate::navlayout`].
    pub fn layout(&self, rows: u16, log_len: usize, panes: usize) -> NavLayout {
        navlayout::layout(rows, self.git.info().is_some(), log_len, panes)
    }

    /// The sidebar's drawn layer: sub-cell [`Paint`] for the charts, in the
    /// card interior's own cell coordinates. `aspect` is the frame's
    /// `cell_h / cell_w`, without which a circle would come out an ellipse and
    /// a chart's proportions would change with the font.
    ///
    /// Kept beside [`Self::cells`] rather than folded into it because the two
    /// layers are drawn by different passes; both read the same section
    /// offsets, which is what keeps a chart under the section it belongs to.
    pub fn chart_paint(&self, cols: u16, rows: u16, aspect: f32) -> Vec<Paint> {
        let mut out = self.cpu_chart(cols, rows, aspect);
        // The SYSTEM section's three arc gauges (their text comes from
        // `gauges::render_stats`, which yields the rings the same width test).
        if rows > clock::CLOCK_H + crate::sysdials::ROWS {
            out.extend(crate::sysdials::NAV.paint(
                self.sampler.stats(),
                cols,
                clock::CLOCK_H + 1,
                aspect,
            ));
        }
        // The NET twin chart, under that section's rates.
        let net_off =
            clock::CLOCK_H + navlayout::SYS_BLOCK + navlayout::LOAD_BLOCK + navlayout::CARD_BLOCK;
        if rows > net_off + 1 + crate::nettwin::ROWS {
            let (rx, tx) = self.sampler.net_dirs();
            out.extend(crate::nettwin::paint(
                rx,
                tx,
                cols,
                net_off + 2,
                aspect,
                crate::net::spark(),
                crate::net::up_color(),
            ));
        }
        out
    }

    /// The ceiling the CPU chart is currently drawn against, in percent, or
    /// `None` when there is no history to scale. The number the SYSTEM rule
    /// writes down: a chart with a moving ceiling and no ceiling written down
    /// is a shape you cannot read a value off.
    fn cpu_ceiling(&self, cols: u16) -> Option<u64> {
        let span = cols.saturating_sub(4) as usize * 2;
        (!self.cpu_hist.is_empty()).then(|| self.cpu_hist.peak(span).max(CHART_FLOOR))
    }

    /// The SYSTEM section's CPU history chart.
    ///
    /// Scaled to the window's own peak with [`CHART_FLOOR`] under it, not to a
    /// flat 0–100. The gauge directly above already answers "how loaded, out
    /// of everything" — pinned to 100 the chart under it could only repeat
    /// that answer, and on a laptop that idles under 10% it repeated it as a
    /// two-pixel smear along the bottom with no shape at all. Against a
    /// rolling peak the same machine draws the *shape* of its last minute,
    /// which is the question the gauge cannot answer, and the floor keeps a
    /// quiet minute from being magnified into a busy-looking one.
    fn cpu_chart(&self, cols: u16, rows: u16, aspect: f32) -> Vec<Paint> {
        let row0 = clock::CLOCK_H + CHART_OFF;
        // Indented under the section legend like the gauges above it, one
        // column of air kept on the right.
        let (col0, width) = (3u16, cols.saturating_sub(4));
        if width == 0 || rows < row0 + CHART_ROWS || self.cpu_hist.is_empty() {
            return Vec::new();
        }
        let span = width as usize * 2;
        // The same ceiling the SYSTEM rule writes down — one derivation, so
        // the number beside the section and the shape under it agree.
        let peak = self.cpu_ceiling(cols).unwrap_or(CHART_FLOOR) as f32;
        let samples: Vec<f32> = self
            .cpu_hist
            .tail(span)
            .into_iter()
            .map(|v| (v as f32 / peak).clamp(0.0, 1.0))
            .collect();
        let mut c = crate::plot::Canvas::new(width, CHART_ROWS, aspect);
        let (w, h) = c.size();
        crate::plot::area::draw(&mut c, (0.0, 0.0, w, h), &samples, crate::palette::accent());
        // The line the curve stands on. Without it a flat trace and an empty
        // block look the same, and the section ends in what reads as a gap.
        c.hairline(0.0, h, w, crew_theme::theme().border_normal, 0.7);
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
            let peak = self.cpu_ceiling(cols);
            for mut c in render_stats(self.sampler.stats(), cols, rows - sys_off, peak) {
                c.row += sys_off;
                out.push(c);
            }
        }
        // The CPU history chart lives below the three gauges. It is *drawn*,
        // not spelled — see `chart_paint`; nothing is emitted here.

        let load_off = clock::CLOCK_H + navlayout::SYS_BLOCK;
        if rows > load_off + 1 {
            let (one, five, fifteen) = load::load_avg();
            for mut c in load::load_cells(one, five, fifteen, load::cores(), cols) {
                c.row += load_off;
                out.push(c);
            }
        }

        let host_off = load_off + navlayout::LOAD_BLOCK;
        if rows > host_off + 3 {
            let (name, uptime) = host::host_strings();
            for mut c in host::host_cells(&name, &uptime, cols) {
                c.row += host_off;
                out.push(c);
            }
        }

        let net_off = host_off + navlayout::CARD_BLOCK;
        if rows > net_off + 3 {
            let s = self.sampler.stats();
            let (rxh, txh) = self.sampler.net_dirs();
            let ceiling = crate::nettwin::ceiling(rxh, txh, cols);
            for mut c in net::net_cells(s.net_rx, s.net_tx, ceiling, cols) {
                c.row += net_off;
                out.push(c);
            }
        }

        let git_off = net_off + navlayout::NET_BLOCK;
        if let Some(info) = self.git.info() {
            if rows > git_off + 3 {
                for mut c in git::git_cells(info, cols) {
                    c.row += git_off;
                    out.push(c);
                }
            }
        }

        // LIVE LOG: recent status messages in their own section, above the
        // panes, as many lines as the nav has rows to spare for them.
        let l = self.layout(rows, log.len(), panes.len());
        if l.log_lines > 0 {
            for mut c in navlog::log_cells(log, cols, l.log_lines, log_back) {
                c.row += l.log_top;
                out.push(c);
            }
        }
        let panes_off = l.panes_top;

        // PANES list fills the remaining height below the LOG section
        // (header + one row per pane).
        const LIST_OFF: u16 = 1;
        if !panes.is_empty() && rows > panes_off + LIST_OFF {
            let limit = (rows - panes_off - LIST_OFF) as usize;
            // A busy pane's row spins; with motion off it holds one frame.
            // The row still says "working" — the glyph is not the animation,
            // the animation is — and a nav that spins through a reduce-motion
            // request is spinning where it was asked not to.
            let spin = crate::update::spinner_frame(crate::anim::now_ms());
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
#[path = "statspane_tests.rs"]
mod tests;

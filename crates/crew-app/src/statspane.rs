use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crew_render::CellView;

use crate::clock;
use crate::gauges::render_stats;
use crate::git::{self, GitWatch};
use crate::host;
use crate::load;
use crate::navlog;
use crate::net;
use crate::panelist::{self, PaneRow};
use crate::stats::SysSampler;

/// Rows the SYSTEM section occupies (rule + 3 gauges + CPU sparkline + gap).
const SYS_BLOCK: u16 = 6;
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

    /// The cell-row where the PANES section header sits — used to hit-test
    /// clicks on the pane list. Must track the section offsets in `cells`,
    /// including the conditional GIT and LOG blocks (`log_len` = buffered
    /// entries, so the caller passes `app.log.len()`).
    pub fn panes_top(&self, log_len: usize) -> u16 {
        let stats = clock::CLOCK_H + SYS_BLOCK + LOAD_BLOCK + CARD_BLOCK + CARD_BLOCK;
        let git = if self.git.info().is_some() {
            CARD_BLOCK
        } else {
            0
        };
        stats + git + navlog::log_block(log_len)
    }

    pub fn cells(&self, cols: u16, rows: u16, panes: &[PaneRow], log: &[String]) -> Vec<CellView> {
        let (time, date) = clock::now_strings();
        let mut out = clock::clock_cells(&time, &date, cols);

        let sys_off = clock::CLOCK_H;
        if rows > sys_off {
            for mut c in render_stats(self.sampler.stats(), cols, rows - sys_off) {
                c.row += sys_off;
                out.push(c);
            }
        }
        // a moving CPU sparkline on the row below the three gauges
        if rows > sys_off + 4 {
            out.extend(crate::spark::cpu_row(&self.cpu_hist, cols, sys_off + 4));
        }

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
            for mut c in navlog::log_cells(log, cols, fit) {
                c.row += next;
                out.push(c);
            }
        }
        let panes_off = next + log_h;

        // PANES list fills the remaining height below the LOG section.
        if !panes.is_empty() && rows > panes_off + 1 {
            let limit = (rows - panes_off - 1) as usize;
            for mut c in panelist::pane_cells(panes, cols, limit) {
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
        // clock(4) + system(6) + load(3) + host(4) + net(4) = 21
        assert_eq!(s.panes_top(0), 21);
        s.git.set_info(Some(git::GitInfo {
            branch: "main".into(),
            changed: 0,
            ahead: 0,
            behind: 0,
        }));
        assert_eq!(s.panes_top(0), 25); // + git(4)
                                        // a non-empty log adds its block: rule + min(n, LOG_LINES) + gap.
        assert_eq!(s.panes_top(2), 25 + 4); // 2 entries -> 2 + 2
        assert_eq!(s.panes_top(99), 25 + navlog::LOG_LINES as u16 + 2); // capped
    }
}

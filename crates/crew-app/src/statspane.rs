use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crew_render::CellView;

use crate::clock;
use crate::gauges::render_stats;
use crate::git::{self, GitInfo};
use crate::host;
use crate::load;
use crate::net;
use crate::panelist::{self, PaneRow};
use crate::stats::SysSampler;

/// Rows the SYSTEM section occupies (rule + 3 gauges + a one-row gap below it).
const SYS_BLOCK: u16 = 5;
/// Rows the LOAD section occupies (rule + 1 line + a one-row gap below it).
const LOAD_BLOCK: u16 = 3;
/// Rows a section with a rule + 2 content rows + one-row gap occupies (HOST, NET, GIT).
const CARD_BLOCK: u16 = 4;
/// Minimum seconds between git queries while the directory is unchanged.
const GIT_POLL_SECS: u64 = 3;

/// The docked sidebar: a live clock card stacked above the system-stats card.
pub struct StatsPane {
    sampler: SysSampler,
    /// Last wall-clock second shown, so the clock repaints once per second.
    last_sec: u64,
    /// Cached git status for the working directory (None = not a repo).
    git: Option<GitInfo>,
    /// Directory the cached git status is for, and when it was last queried.
    git_cwd: PathBuf,
    git_sec: u64,
}

impl StatsPane {
    pub fn new() -> Self {
        Self {
            sampler: SysSampler::new(),
            last_sec: 0,
            git: None,
            git_cwd: PathBuf::new(),
            git_sec: 0,
        }
    }

    /// Returns true when the sidebar should repaint — fresh stats (~1s throttle),
    /// a new wall-clock second for the clock, or changed git status for `cwd`.
    pub fn refresh(&mut self, cwd: &Path) -> bool {
        let stats_changed = self.sampler.refresh();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let clock_changed = now != self.last_sec;
        self.last_sec = now;
        let git_changed = self.refresh_git(cwd, now);
        stats_changed || clock_changed || git_changed
    }

    /// Re-query git when the directory changed or the poll interval elapsed.
    /// Returns true when the cached status actually changed.
    fn refresh_git(&mut self, cwd: &Path, now: u64) -> bool {
        let moved = self.git_cwd != cwd;
        if !moved && now.saturating_sub(self.git_sec) < GIT_POLL_SECS {
            return false;
        }
        self.git_cwd = cwd.to_path_buf();
        self.git_sec = now;
        let fresh = git::query(cwd);
        if fresh != self.git {
            self.git = fresh;
            return true;
        }
        false
    }

    /// The cell-row where the PANES section header sits — used to hit-test
    /// clicks on the pane list. Must track the section offsets in `cells`.
    pub fn panes_top(&self) -> u16 {
        let stats = clock::CLOCK_H + SYS_BLOCK + LOAD_BLOCK + CARD_BLOCK + CARD_BLOCK;
        stats + if self.git.is_some() { CARD_BLOCK } else { 0 }
    }

    pub fn cells(&self, cols: u16, rows: u16, panes: &[PaneRow]) -> Vec<CellView> {
        let (time, date) = clock::now_strings();
        let mut out = clock::clock_cells(&time, &date, cols);

        let sys_off = clock::CLOCK_H;
        if rows > sys_off {
            for mut c in render_stats(self.sampler.stats(), cols, rows - sys_off) {
                c.row += sys_off;
                out.push(c);
            }
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
            for mut c in net::net_cells(s.net_rx, s.net_tx, cols) {
                c.row += net_off;
                out.push(c);
            }
        }

        let git_off = net_off + CARD_BLOCK;
        let mut next = git_off;
        if let Some(info) = &self.git {
            if rows > git_off + 3 {
                for mut c in git::git_cells(info, cols) {
                    c.row += git_off;
                    out.push(c);
                }
            }
            next = git_off + CARD_BLOCK; // only reserve the GIT block when shown
        }

        // PANES list fills the remaining height below the last stat section.
        if !panes.is_empty() && rows > next + 1 {
            let limit = (rows - next - 1) as usize;
            for mut c in panelist::pane_cells(panes, cols, limit) {
                c.row += next;
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
    fn panes_top_accounts_for_git() {
        let mut s = StatsPane::new();
        // clock(4) + system(5) + load(3) + host(4) + net(4) = 20
        assert_eq!(s.panes_top(), 20);
        s.git = Some(GitInfo {
            branch: "main".into(),
            dirty: false,
            ahead: 0,
            behind: 0,
        });
        assert_eq!(s.panes_top(), 24); // + git(4)
    }
}

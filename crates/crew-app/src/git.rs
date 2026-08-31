//! Sidebar git section: a `GIT` divider above the current branch and a clean/
//! dirty marker for Crew's working directory. Queried with the `git` CLI (so it
//! honours the user's full git config) and cached/throttled by `StatsPane`.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use crew_render::CellView;

use crate::boxdraw::section_header;

use crate::palette::accent;
use crew_hive::childproc::no_console_window;

/// Minimum seconds between git queries while the working directory is unchanged.
const GIT_POLL_SECS: u64 = 3;

/// Branch name, number of changed files, and commits ahead/behind the upstream.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GitInfo {
    pub branch: String,
    pub changed: usize,
    pub ahead: usize,
    pub behind: usize,
}

/// Query `git` for `dir`'s branch, changed-file count, and ahead/behind in a
/// single `status --porcelain --branch`. `None` when `dir` isn't a repo (no `git`).
pub fn query(dir: &Path) -> Option<GitInfo> {
    let out = run(dir, &["status", "--porcelain", "--branch"])?;
    parse_status(&out)
}

/// Throttled, off-the-main-thread git status for the sidebar. `query` shells out
/// to `git status`, which can take seconds in a large or network-mounted repo;
/// running it inline on the winit event loop froze rendering and input in every
/// pane. `GitWatch` instead spawns each query on a background thread and hands
/// the result back through a channel, so `poll` never blocks the UI.
#[derive(Default)]
pub struct GitWatch {
    /// Directory the cached status is for.
    cwd: PathBuf,
    /// Unix second the in-flight/last query was launched (0 = none yet, forces a
    /// query on the next poll).
    sec: u64,
    /// Last known status (None = not a repo, or not yet queried).
    info: Option<GitInfo>,
    /// Result channel for a query currently running on a background thread.
    rx: Option<Receiver<(PathBuf, Option<GitInfo>)>>,
}

impl GitWatch {
    /// The most recent status, if any.
    pub fn info(&self) -> Option<&GitInfo> {
        self.info.as_ref()
    }

    /// Seed the cached status directly (layout tests that don't run a query).
    #[cfg(test)]
    pub(crate) fn set_info(&mut self, info: Option<GitInfo>) {
        self.info = info;
    }

    /// Non-blocking. Harvests a finished background query, then launches a fresh
    /// one when the directory changed or the poll interval elapsed and none is
    /// already running. Returns true when the cached status actually changed.
    pub fn poll(&mut self, cwd: &Path, now: u64) -> bool {
        self.poll_with(cwd, now, query)
    }

    /// `poll` with an injectable query function, for tests.
    fn poll_with<F>(&mut self, cwd: &Path, now: u64, q: F) -> bool
    where
        F: Fn(&Path) -> Option<GitInfo> + Send + 'static,
    {
        let mut changed = false;
        // 1. Harvest a finished query (non-blocking). Ignore a stale result for a
        //    directory we've since moved away from.
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok((dir, info)) => {
                    self.rx = None;
                    if dir == self.cwd && info != self.info {
                        self.info = info;
                        changed = true;
                    }
                }
                Err(TryRecvError::Empty) => {} // still running
                Err(TryRecvError::Disconnected) => self.rx = None,
            }
        }
        // 2. A directory change forces a fresh query for the new directory.
        if self.cwd != cwd {
            self.cwd = cwd.to_path_buf();
            self.sec = 0;
        }
        // 3. Launch a query when due and none is already in flight.
        let due = self.sec == 0 || now.saturating_sub(self.sec) >= GIT_POLL_SECS;
        if due && self.rx.is_none() {
            self.sec = now.max(1); // keep 0 reserved as the "force" sentinel
            let (tx, rx) = mpsc::channel();
            let dir = self.cwd.clone();
            std::thread::spawn(move || {
                let info = q(&dir);
                let _ = tx.send((dir, info));
            });
            self.rx = Some(rx);
        }
        changed
    }
}

/// Parse `git status --porcelain --branch` output: the `## …` header gives the
/// branch and ahead/behind, and each further line is one changed file.
fn parse_status(out: &str) -> Option<GitInfo> {
    let mut lines = out.lines();
    let header = lines.next()?;
    let branch = parse_branch(header)?;
    let changed = lines.filter(|l| !l.trim().is_empty()).count();
    let (ahead, behind) = parse_ahead_behind(header);
    Some(GitInfo {
        branch,
        changed,
        ahead,
        behind,
    })
}

/// Branch name from a `## branch...upstream [ahead N, behind M]` header.
fn parse_branch(header: &str) -> Option<String> {
    let rest = header.strip_prefix("## ")?;
    let name = rest.split("...").next().unwrap_or(rest);
    let name = name.split(" [").next().unwrap_or(name);
    Some(name.trim().to_string())
}

/// `(ahead, behind)` counts parsed from the `[ahead N, behind M]` suffix.
fn parse_ahead_behind(header: &str) -> (usize, usize) {
    let inside = header
        .split_once('[')
        .and_then(|(_, r)| r.split_once(']'))
        .map(|(s, _)| s)
        .unwrap_or("");
    (grab(inside, "ahead "), grab(inside, "behind "))
}

/// The number following `key` in `s` (0 if absent).
fn grab(s: &str, key: &str) -> usize {
    s.split_once(key)
        .and_then(|(_, r)| r.trim_start().split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// Run `git -C dir <args>`, returning trimmed stdout on success.
fn run(dir: &Path, args: &[&str]) -> Option<String> {
    let out = no_console_window(&mut Command::new("git"))
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Render the GIT section: a `GIT` rule on row 0, the branch on row 1, and a
/// clean/dirty marker on row 2.
pub fn git_cells(info: &GitInfo, cols: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = section_header("GIT", cols, t.border_normal, accent(), t.page_bg);
    let mut head = info.branch.clone();
    if info.ahead > 0 {
        head.push_str(&format!(" ↑{}", info.ahead));
    }
    if info.behind > 0 {
        head.push_str(&format!(" ↓{}", info.behind));
    }
    put(&mut out, &head, 1, cols, t.ink, t.page_bg);
    let (marker, fg) = if info.changed > 0 {
        (format!("● {} changed", info.changed), t.status_fg)
    } else {
        ("✓ clean".to_string(), t.text_muted)
    };
    put(&mut out, &marker, 2, cols, fg, t.page_bg);
    out
}

/// Draw `s` at `row`, indented to align under the section legend, clipped to `cols`.
fn put(out: &mut Vec<CellView>, s: &str, row: u16, cols: u16, fg: (u8, u8, u8), bg: (u8, u8, u8)) {
    let max = cols.saturating_sub(4) as usize;
    for (i, c) in s.chars().take(max).enumerate() {
        out.push(CellView {
            col: 3 + i as u16,
            row,
            c,
            fg,
            bg,
            bold: false,
            italic: false,
            ..Default::default()
        });
    }
}

#[cfg(test)]
#[path = "git_tests.rs"]
mod tests;

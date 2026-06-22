//! Sidebar git section: a `GIT` divider above the current branch and a clean/
//! dirty marker for Crew's working directory. Queried with the `git` CLI (so it
//! honours the user's full git config) and cached/throttled by `StatsPane`.
use std::path::Path;
use std::process::Command;

use crew_render::CellView;

use crate::boxdraw::section_header;

const ACCENT: (u8, u8, u8) = (0, 255, 160);
const LABEL: (u8, u8, u8) = (200, 200, 200);
const DIM: (u8, u8, u8) = (150, 150, 160);
const BORDER: (u8, u8, u8) = (110, 110, 120);
const DIRTY: (u8, u8, u8) = (230, 180, 90);
const BG: (u8, u8, u8) = (0, 0, 0);

/// Branch name, dirty state, and commits ahead/behind the upstream.
#[derive(Clone, PartialEq, Eq)]
pub struct GitInfo {
    pub branch: String,
    pub dirty: bool,
    pub ahead: usize,
    pub behind: usize,
}

/// Query `git` for `dir`'s branch, dirty state, and ahead/behind in a single
/// `status --porcelain --branch`. `None` when `dir` isn't a repo (or no `git`).
pub fn query(dir: &Path) -> Option<GitInfo> {
    let out = run(dir, &["status", "--porcelain", "--branch"])?;
    parse_status(&out)
}

/// Parse `git status --porcelain --branch` output: the `## …` header gives the
/// branch and ahead/behind, any further lines mean the tree is dirty.
fn parse_status(out: &str) -> Option<GitInfo> {
    let mut lines = out.lines();
    let header = lines.next()?;
    let branch = parse_branch(header)?;
    let dirty = lines.any(|l| !l.trim().is_empty());
    let (ahead, behind) = parse_ahead_behind(header);
    Some(GitInfo {
        branch,
        dirty,
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
    let out = Command::new("git")
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
    let mut out = section_header("GIT", cols, BORDER, ACCENT, BG);
    let mut head = info.branch.clone();
    if info.ahead > 0 {
        head.push_str(&format!(" ↑{}", info.ahead));
    }
    if info.behind > 0 {
        head.push_str(&format!(" ↓{}", info.behind));
    }
    put(&mut out, &head, 1, cols, LABEL);
    let (marker, fg) = if info.dirty {
        ("● uncommitted", DIRTY)
    } else {
        ("✓ clean", DIM)
    };
    put(&mut out, marker, 2, cols, fg);
    out
}

/// Draw `s` at `row`, indented to align under the section legend, clipped to `cols`.
fn put(out: &mut Vec<CellView>, s: &str, row: u16, cols: u16, fg: (u8, u8, u8)) {
    let max = cols.saturating_sub(4) as usize;
    for (i, c) in s.chars().take(max).enumerate() {
        out.push(CellView {
            col: 3 + i as u16,
            row,
            c,
            fg,
            bg: BG,
            bold: false,
            italic: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_non_repo_is_none() {
        let dir = std::env::temp_dir().join("crew_git_not_a_repo");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(query(&dir).is_none());
    }

    #[test]
    fn git_cells_show_branch_and_marker() {
        let info = GitInfo {
            branch: "main".into(),
            dirty: true,
            ahead: 2,
            behind: 0,
        };
        let cells = git_cells(&info, 24);
        // GIT divider on row 0
        assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
        assert!(cells.iter().any(|c| c.c == 'G' && c.row == 0));
        // branch + ahead arrow on row 1
        assert!(cells.iter().any(|c| c.c == 'm' && c.row == 1));
        assert!(cells.iter().any(|c| c.c == '↑' && c.row == 1));
        // dirty marker (amber) on row 2
        assert!(cells
            .iter()
            .any(|c| c.c == '●' && c.row == 2 && c.fg == DIRTY));
    }

    #[test]
    fn git_cells_clean_marker() {
        let info = GitInfo {
            branch: "dev".into(),
            dirty: false,
            ahead: 0,
            behind: 0,
        };
        let cells = git_cells(&info, 24);
        assert!(cells.iter().any(|c| c.c == '✓' && c.row == 2));
    }

    #[test]
    fn parse_status_reads_branch_dirty_ahead_behind() {
        let out = "## main...origin/main [ahead 1, behind 2]\n M src/x.rs\n?? new\n";
        let info = parse_status(out).unwrap();
        assert_eq!(info.branch, "main");
        assert!(info.dirty);
        assert_eq!((info.ahead, info.behind), (1, 2));
    }

    #[test]
    fn parse_status_clean_no_upstream() {
        let info = parse_status("## feature/x\n").unwrap();
        assert_eq!(info.branch, "feature/x");
        assert!(!info.dirty);
        assert_eq!((info.ahead, info.behind), (0, 0));
    }

    #[test]
    fn parse_status_ahead_only() {
        let info = parse_status("## main...up [ahead 3]\n").unwrap();
        assert_eq!((info.ahead, info.behind), (3, 0));
    }
}

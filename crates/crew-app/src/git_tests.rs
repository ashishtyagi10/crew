use super::*;
use std::time::{Duration, Instant};

/// A git query that takes 300ms — stands in for a slow/large repo.
fn slow_query(_dir: &Path) -> Option<GitInfo> {
    std::thread::sleep(Duration::from_millis(300));
    Some(GitInfo {
        branch: "slow".into(),
        changed: 0,
        ahead: 0,
        behind: 0,
    })
}

#[test]
fn poll_does_not_block_on_a_slow_query() {
    let mut w = GitWatch::default();
    let dir = std::env::temp_dir();

    // The first poll only *launches* the background query; it must return
    // promptly even though the query itself takes 300ms.
    let start = Instant::now();
    let changed = w.poll_with(&dir, 1, slow_query);
    assert!(
        start.elapsed() < Duration::from_millis(100),
        "poll blocked on the slow query for {:?}",
        start.elapsed()
    );
    assert!(
        !changed,
        "no result should be available on the launching poll"
    );
    assert!(w.info().is_none());

    // A later poll, after the background query finishes, picks up the result.
    let mut got = false;
    for t in 0..100 {
        std::thread::sleep(Duration::from_millis(20));
        if w.poll_with(&dir, 1 + t as u64, slow_query) {
            got = true;
            break;
        }
    }
    assert!(got, "background git result was never harvested");
    assert_eq!(w.info().map(|g| g.branch.as_str()), Some("slow"));
}

#[test]
fn query_non_repo_is_none() {
    let dir = std::env::temp_dir().join("crew_git_not_a_repo");
    std::fs::create_dir_all(&dir).unwrap();
    assert!(query(&dir).is_none());
}

#[test]
fn git_cells_show_branch_and_marker() {
    let _g = crate::app::theme_test_guard();
    let info = GitInfo {
        branch: "main".into(),
        changed: 2,
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
    // changed-count marker (amber/status) on row 2, with the count
    assert!(cells
        .iter()
        .any(|c| c.c == '●' && c.row == 2 && c.fg == crew_theme::theme().status_fg));
    assert!(cells.iter().any(|c| c.c == '2' && c.row == 2));
}

#[test]
fn git_cells_clean_marker() {
    let info = GitInfo {
        branch: "dev".into(),
        changed: 0,
        ahead: 0,
        behind: 0,
    };
    let cells = git_cells(&info, 24);
    assert!(cells.iter().any(|c| c.c == '✓' && c.row == 2));
}

#[test]
fn parse_status_reads_branch_changed_ahead_behind() {
    let out = "## main...origin/main [ahead 1, behind 2]\n M src/x.rs\n?? new\n";
    let info = parse_status(out).unwrap();
    assert_eq!(info.branch, "main");
    assert_eq!(info.changed, 2);
    assert_eq!((info.ahead, info.behind), (1, 2));
}

#[test]
fn parse_status_clean_no_upstream() {
    let info = parse_status("## feature/x\n").unwrap();
    assert_eq!(info.branch, "feature/x");
    assert_eq!(info.changed, 0);
    assert_eq!((info.ahead, info.behind), (0, 0));
}

#[test]
fn parse_status_ahead_only() {
    let info = parse_status("## main...up [ahead 3]\n").unwrap();
    assert_eq!((info.ahead, info.behind), (3, 0));
}

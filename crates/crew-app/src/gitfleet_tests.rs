use super::*;

fn p(s: &str) -> PathBuf {
    PathBuf::from(s)
}

#[test]
fn a_directory_never_asked_about_is_due_at_once() {
    let dirs = vec![p("/a"), p("/b")];
    assert_eq!(next_due(&HashMap::new(), &dirs, 100), Some(p("/a")));
}

/// The stalest one goes first, so a fleet of panes takes turns instead of one
/// directory holding the single query slot.
#[test]
fn the_longest_waiting_directory_goes_next() {
    let asked = HashMap::from([(p("/a"), 90u64), (p("/b"), 50), (p("/c"), 70)]);
    let dirs = vec![p("/a"), p("/b"), p("/c")];
    assert_eq!(next_due(&asked, &dirs, 100), Some(p("/b")));
}

/// A directory asked about a moment ago is not asked again — `git status` is
/// the expensive part, and a pane's repo does not change every frame.
#[test]
fn nothing_is_due_until_the_throttle_has_passed() {
    let asked = HashMap::from([(p("/a"), 100u64)]);
    let dirs = vec![p("/a")];
    assert_eq!(next_due(&asked, &dirs, 100), None);
    assert_eq!(next_due(&asked, &dirs, 100 + POLL_SECS - 1), None);
    assert_eq!(next_due(&asked, &dirs, 100 + POLL_SECS), Some(p("/a")));
}

/// A directory no pane is in any more is not scheduled, whatever its history.
#[test]
fn a_closed_panes_directory_is_not_asked_about() {
    let asked = HashMap::from([(p("/gone"), 0u64)]);
    assert_eq!(next_due(&asked, &[p("/here")], 100), Some(p("/here")));
    assert_eq!(next_due(&asked, &[], 100), None);
}

/// Answers and schedule both drop directories no pane holds, so a long
/// session's map stays the size of the fleet rather than of its history.
#[test]
fn polling_forgets_directories_that_left_the_fleet() {
    let mut f = GitFleet::default();
    f.known.insert(p("/old"), None);
    f.asked.insert(p("/old"), 1);
    f.poll(&[], 100);
    assert!(f.known.is_empty() && f.asked.is_empty());
    assert!(f.rx.is_none(), "an empty fleet started a query");
}

#[test]
fn nothing_is_known_about_a_directory_before_the_first_answer() {
    let mut f = GitFleet::default();
    assert_eq!(f.info(Some(&p("/a"))), None);
    assert_eq!(f.info(None), None);
    f.known.insert(
        p("/a"),
        Some(GitInfo {
            branch: "main".into(),
            changed: 0,
            ahead: 0,
            behind: 0,
        }),
    );
    assert_eq!(
        f.info(Some(&p("/a"))).map(|g| g.branch.as_str()),
        Some("main")
    );
    f.known.insert(p("/b"), None);
    assert_eq!(f.info(Some(&p("/b"))), None, "not a repo is not an answer");
}

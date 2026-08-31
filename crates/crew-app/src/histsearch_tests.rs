use super::{next_match, prev_match};

fn hist() -> Vec<String> {
    vec!["git status".into(), "ls -la".into(), "git push".into()]
}

#[test]
fn prefix_filters_to_matching_entries() {
    let h = hist();
    // newest "git" before the end is index 2, then 0 (skipping "ls -la").
    assert_eq!(prev_match(&h, "git", h.len()), Some(2));
    assert_eq!(prev_match(&h, "git", 2), Some(0));
    assert_eq!(prev_match(&h, "git", 0), None);
    // forward from 0 finds 2; nothing newer than 2.
    assert_eq!(next_match(&h, "git", 0), Some(2));
    assert_eq!(next_match(&h, "git", 2), None);
}

#[test]
fn empty_prefix_matches_everything() {
    let h = hist();
    assert_eq!(prev_match(&h, "", h.len()), Some(2));
    assert_eq!(prev_match(&h, "", 2), Some(1));
    assert_eq!(next_match(&h, "", 0), Some(1));
    // empty history never matches.
    assert_eq!(prev_match(&[], "x", 0), None);
}

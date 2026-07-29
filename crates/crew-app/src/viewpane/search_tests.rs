use super::*;

#[test]
fn matches_are_line_indexes_in_order() {
    let lines = ["alpha", "beta", "alpha again"];
    assert_eq!(find_matches(&lines, "alpha"), vec![0, 2]);
}

#[test]
fn matching_ignores_case() {
    let lines = ["Alpha", "BETA"];
    assert_eq!(find_matches(&lines, "beta"), vec![1]);
}

#[test]
fn an_empty_needle_matches_nothing() {
    // Otherwise every line "matches" and n/N walks the whole file uselessly.
    let lines = ["a", "b"];
    assert!(find_matches(&lines, "").is_empty());
}

#[test]
fn next_wraps_at_the_end_and_prev_wraps_at_the_start() {
    let mut s = Search::new("x".into(), vec![2, 7]);
    assert_eq!(s.next(), Some(2));
    assert_eq!(s.next(), Some(7));
    assert_eq!(s.next(), Some(2), "wraps forward");
    assert_eq!(s.prev(), Some(7), "wraps backward");
}

#[test]
fn a_search_with_no_hits_reports_none() {
    let mut s = Search::new("zzz".into(), Vec::new());
    assert_eq!(s.next(), None);
    assert_eq!(s.prev(), None);
}

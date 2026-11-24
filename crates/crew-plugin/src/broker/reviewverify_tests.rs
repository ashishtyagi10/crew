//! The second look keeps, withdraws, or — silent — leaves the findings be.
use super::*;

#[test]
fn keep_and_drop_lines_are_read_per_finding() {
    let r = "1 keep\n2. drop — the guard is on line 3\n- 3: keep\nnoise";
    let v = withdrawn(r, 3).unwrap();
    assert_eq!(v, [None, Some("the guard is on line 3".into()), None]);
    assert_eq!(withdrawn("looks fine to me", 2), None);
    assert_eq!(withdrawn("9 drop — out of range", 2), None);
}

#[test]
fn the_prompt_numbers_the_findings_and_carries_the_code() {
    let p = prompt("DIFFBODY", "CALLERS x", &["- warn — a.rs:1 — w"]);
    assert!(p.contains("1. - warn — a.rs:1 — w"), "{p}");
    assert!(p.contains("DIFFBODY") && p.contains("CALLERS x"), "{p}");
    assert!(p.contains("`N keep` or `N drop — why`"), "{p}");
}

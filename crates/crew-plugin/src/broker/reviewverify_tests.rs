//! The second look keeps, withdraws, or — silent — leaves the findings be.
use super::*;

#[test]
fn keep_and_drop_lines_are_read_per_finding() {
    let r = "1 keep\n2. drop — the guard is on line 3\n- 3: keep\nnoise";
    let v = withdrawn(r, 3).unwrap();
    assert_eq!(
        v,
        [
            Look::Kept,
            Look::Dropped("the guard is on line 3".into()),
            Look::Kept
        ]
    );
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

#[test]
fn the_prompt_s_own_dash_and_bold_numbers_are_read() {
    let r = "1 — drop — no such guard\n2 - keep\n**3** drop\n4 – keep";
    let v = withdrawn(r, 5).unwrap();
    assert_eq!(
        v,
        [
            Look::Dropped("no such guard".into()),
            Look::Kept,
            Look::Dropped(String::new()),
            Look::Kept,
            Look::Unanswered,
        ]
    );
}

//! The closing answer's brief carries how each worker's output ENDED.
//!
//! A worker writes findings first and its verdict last. The brief used to
//! keep only the head of a long output, so the answer read the findings and
//! never the verdict; these hold the tail in.
use super::*;

/// What a worker's output looks like: 200 findings, and the verdict last.
fn findings_then_verdict() -> String {
    let mut s: String = (1..=200)
        .map(|n| format!("finding {n}: looked at one more thing\n"))
        .collect();
    s.push_str("VERDICT: the fix is in route.rs:412");
    s
}

/// (e) A long output's last line reaches the closing answer's brief, its
/// first line with it, and the cut between them says so.
#[test]
fn the_brief_of_a_long_output_carries_its_last_line() {
    let long = findings_then_verdict();
    assert!(
        long.chars().count() > OUTPUT_CAP,
        "fixture must bust the cap"
    );
    let brief = prompt("where is the bug?", &[("look".into(), long)]);
    assert!(
        brief.ends_with("VERDICT: the fix is in route.rs:412"),
        "the worker's conclusion was cut: {}",
        &brief[brief.len() - 200..]
    );
    assert!(
        brief.contains("## look\nfinding 1: "),
        "and its head is kept"
    );
    assert!(
        brief.contains("chars cut from the middle]"),
        "the cut is said"
    );
}

/// Every output in a wide brief keeps its own ending, not just the last one.
#[test]
fn every_output_in_a_shared_budget_keeps_its_ending() {
    let parts: Vec<(String, String)> = (0..4)
        .map(|i| {
            let body = "a line of what was found\n".repeat(300);
            (format!("t{i}"), format!("{body}conclusion of t{i}"))
        })
        .collect();
    let brief = outputs(&parts);
    for i in 0..4 {
        assert!(
            brief.contains(&format!("conclusion of t{i}")),
            "worker t{i}'s conclusion was cut"
        );
    }
}

use super::*;

// The reviewer is elected by the MODEL (`elect::elect`), with a
// deterministic roster-first fallback — see `elect_tests` for the grammar
// and fallback coverage; the keyword election it replaced is gone.

#[test]
fn review_prompt_carries_the_diff_and_the_reviewer_contract() {
    let p = review_prompt("+ let x = unwrap();", "");
    assert!(p.contains("+ let x = unwrap();"), "diff included");
    let lower = p.to_lowercase();
    for word in ["blocker", "warn", "nit"] {
        assert!(
            lower.contains(word),
            "severity vocabulary names {word}: {p}"
        );
    }
    assert!(lower.contains("file"), "asks findings to name the file");
    assert!(lower.contains("verdict"), "asks for a closing verdict");
    assert!(
        lower.contains("clean") || lower.contains("no findings"),
        "tells the reviewer what to say for a clean diff"
    );
}

#[test]
fn review_prompt_orders_findings_by_severity() {
    let lower = review_prompt("x", "").to_lowercase();
    let (b, w, n) = (
        lower.find("blocker").unwrap(),
        lower.find("warn").unwrap(),
        lower.find("nit").unwrap(),
    );
    assert!(b < w && w < n, "severities are introduced worst-first");
}

/// Loop 39's S9 probe: a reviewer listed two blockers and then ended on
/// "no findings", because the prompt asked for a verdict line and named
/// "no findings" in the same breath. The verdict now counts what was found,
/// and `no findings` is reserved for a review that found nothing.
#[test]
fn the_verdict_counts_findings_and_no_findings_stands_alone() {
    let p = review_prompt("diff --git a/x b/x", "");
    assert!(p.contains("`verdict: 2 blocker, 1 warn`"), "{p}");
    assert!(p.contains("never under a finding"), "{p}");
}
